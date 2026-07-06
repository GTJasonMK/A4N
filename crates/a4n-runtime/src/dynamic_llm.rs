//! 动态 LLM Provider：每次调用时从数据库解析当前激活的 LLM 配置。

use std::{
    collections::HashMap,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Instant,
};

use a4n_core::config::app_config::{
    DEFAULT_LLM_MAX_CONTEXT_TOKENS, DEFAULT_LLM_MAX_OUTPUT_TOKENS, LlmMaxTokensConfig,
};
use a4n_core::error::AppError;
use a4n_core::models::ai::{llm_config, system_settings};
use a4n_engine::llm::http::HttpLlmProvider;
use a4n_engine::llm::provider::{
    ChatChunk, ChatMessage, ChatRequest, ChatResponse, LlmProvider, LlmTraceContext, MessageRole,
    ModelCapabilities, UsageInfo,
};
use a4n_engine::queue::priority::{TaskPermit, TaskPriority, TaskQueue, TaskType};
use a4n_services::generation::{RecordLlmCallTraceInput, record_llm_call_trace};
use a4n_services::llm_context::{
    COMPRESSION_FULL_RAW, ContextBudgetPlanner, LlmContextManifest, LlmContextPackDraft,
    LlmContextPackDraftInput, LlmContextPackRef, LlmContextPackRequirement, PlannedContextPack,
    context_pack_model_ref, record_or_reuse_context_pack, render_context_manifest_user_content,
    replace_request_user_content_from_manifest, validate_context_manifest,
};
use a4n_services::queue::PersistentTaskQueue;
use async_trait::async_trait;
use futures::stream::Stream;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde_json::json;

use crate::runtime_config;

pub struct DynamicLlmProvider {
    db: Arc<DatabaseConnection>,
    task_queue: Arc<PersistentTaskQueue>,
}

pub const LLM_MAX_TOKENS_SETTING_KEY: &str = "llm_max_tokens";

#[derive(Debug, Clone)]
pub struct EffectiveLlmPreviewBudget {
    pub model_name: String,
    pub max_context_tokens: usize,
    pub max_output_tokens: usize,
    pub output_token_reserve: usize,
    pub output_token_source: String,
}

impl DynamicLlmProvider {
    pub fn new(db: Arc<DatabaseConnection>, task_queue: Arc<PersistentTaskQueue>) -> Self {
        Self { db, task_queue }
    }

    async fn resolve_default(&self) -> Result<HttpLlmProvider, AppError> {
        self.resolve_provider_by_id(None).await
    }

    async fn resolve_model_by_id(
        &self,
        config_id: Option<i64>,
    ) -> Result<llm_config::Model, AppError> {
        if let Some(snapshot_model) = runtime_config::current_llm_model_for_id(config_id) {
            return Ok(snapshot_model);
        }
        if config_id.is_none() && runtime_config::uses_system_default_llm_template() {
            return self.resolve_system_default_template_with_user_key().await;
        }
        if let Some(id) = config_id
            && runtime_config::current_llm_snapshot_scope_present()
        {
            return Err(AppError::ConfigError(format!(
                "runtime_config_snapshot.llm_configs 不包含 LLM 配置 #{id}"
            )));
        }
        if runtime_config::current_user_id().is_some() {
            return Err(AppError::ConfigError(match config_id {
                Some(id) => format!(
                    "远程用户请求不能直接使用服务器 LLM 配置 #{id}；请在用户配置页使用个人 LLM 配置快照"
                ),
                None => "使用系统默认 LLM 需要先在用户配置页填写并保存你的个人 API Key".to_string(),
            }));
        }

        crate::global_ai_config::resolve_llm_config(self.db.as_ref(), config_id).await
    }

    async fn resolve_system_default_template_with_user_key(
        &self,
    ) -> Result<llm_config::Model, AppError> {
        let api_key = runtime_config::current_system_llm_api_key().ok_or_else(|| {
            AppError::ConfigError(
                "使用系统默认 LLM 需要先在用户配置页填写并保存你的个人 API Key".to_string(),
            )
        })?;
        let mut model = crate::global_ai_config::resolve_llm_config(self.db.as_ref(), None).await?;
        model.api_key_encrypted = Some(api_key);
        Ok(model)
    }

    async fn resolve_provider_by_id(
        &self,
        config_id: Option<i64>,
    ) -> Result<HttpLlmProvider, AppError> {
        let model = self.resolve_model_by_id(config_id).await?;
        make_provider(&model)
    }

    async fn resolve_max_tokens(&self, request: &ChatRequest) -> Result<Option<usize>, AppError> {
        let Some(profile) = request.max_tokens_profile.as_deref() else {
            return Ok(request.max_tokens);
        };
        let config = match runtime_config::current_llm_max_tokens() {
            Some(config) => config,
            None => load_llm_max_tokens_config(self.db.as_ref()).await?,
        };
        let configured = config.for_profile(profile).ok_or_else(|| {
            AppError::ConfigError(format!("未知的 LLM max_tokens 场景配置: {profile}"))
        })?;
        Ok(Some(match request.max_tokens {
            Some(required_minimum) => configured.max(required_minimum),
            None => configured,
        }))
    }

    pub async fn chat_with_config_id(
        &self,
        config_id: Option<i64>,
        mut request: ChatRequest,
    ) -> Result<ChatResponse, AppError> {
        let profile = request.max_tokens_profile.clone();
        let temperature = request.temperature;
        let message_count = request.messages.len();
        let prompt_chars: usize = request.messages.iter().map(|msg| msg.content.len()).sum();
        let provider = self.resolve_provider_by_id(config_id).await?;
        let model_name = provider.model_name().to_string();
        let model_max_output_tokens = provider.capabilities().max_output_tokens;
        request.max_tokens = self.resolve_max_tokens(&request).await?;
        ensure_max_tokens_within_model_cap(
            request.max_tokens,
            profile.as_deref(),
            model_max_output_tokens,
            model_name.as_str(),
        )?;
        let trace_context =
            prepare_dynamic_llm_trace_context(self.db.as_ref(), &provider, &mut request).await?;
        tracing::debug!(
            target: "a4n::llm",
            model = %model_name,
            config_id = ?config_id,
            profile = ?profile,
            max_tokens = ?request.max_tokens,
            temperature = ?temperature,
            message_count,
            prompt_chars,
            "LLM chat 请求"
        );
        let started = Instant::now();
        let result = self
            .task_queue
            .run_tracked(
                TaskType::LlmCall,
                TaskPriority::Medium,
                None,
                || async move { provider.chat(request).await },
            )
            .await;
        match &result {
            Ok(response) => {
                let response_chars = response.message.content.len();
                record_dynamic_llm_trace(
                    self.db.as_ref(),
                    trace_context.as_ref(),
                    Some(model_name.as_str()),
                    response.usage.clone(),
                    Some(started.elapsed().as_millis() as i64),
                    false,
                    "completed",
                    None,
                    None,
                    response.finish_reason.as_deref(),
                )
                .await;
                tracing::debug!(
                    target: "a4n::llm",
                    model = %model_name,
                    config_id = ?config_id,
                    profile = ?profile,
                    duration_ms = started.elapsed().as_millis(),
                    finish_reason = ?response.finish_reason,
                    usage = ?response.usage,
                    response_chars,
                    "LLM chat 响应"
                );
            }
            Err(error) => {
                let message = error.to_string();
                record_dynamic_llm_trace(
                    self.db.as_ref(),
                    trace_context.as_ref(),
                    Some(model_name.as_str()),
                    None,
                    Some(started.elapsed().as_millis() as i64),
                    false,
                    llm_trace_status_for_error(error),
                    Some(llm_error_kind(error)),
                    Some(message.as_str()),
                    None,
                )
                .await;
                tracing::warn!(
                    target: "a4n::llm",
                    model = %model_name,
                    config_id = ?config_id,
                    profile = ?profile,
                    duration_ms = started.elapsed().as_millis(),
                    error = %error,
                    "LLM chat 失败"
                );
            }
        }
        result
    }
}

pub async fn resolve_effective_llm_preview_budget(
    db: &DatabaseConnection,
    request: &ChatRequest,
) -> Result<EffectiveLlmPreviewBudget, AppError> {
    let model = resolve_preview_llm_model(db).await?;
    let max_context_tokens = usize::try_from(model.max_context_tokens).map_err(|_| {
        AppError::ConfigError(format!(
            "LLM 配置 {} 的 max_context_tokens 必须大于 0",
            model.name
        ))
    })?;
    let max_output_tokens = usize::try_from(model.max_output_tokens).map_err(|_| {
        AppError::ConfigError(format!(
            "LLM 配置 {} 的 max_output_tokens 必须大于 0",
            model.name
        ))
    })?;
    if max_context_tokens == 0 || max_output_tokens == 0 || max_output_tokens >= max_context_tokens
    {
        return Err(AppError::ConfigError(format!(
            "LLM 配置 {} 的模型能力无效：max_context_tokens={}, max_output_tokens={}",
            model.name, max_context_tokens, max_output_tokens
        )));
    }

    let (output_token_reserve, output_token_source) = match request.max_tokens_profile.as_deref() {
        Some(profile) => {
            let config = match runtime_config::current_llm_max_tokens() {
                Some(config) => config,
                None => load_llm_max_tokens_config(db).await?,
            };
            let configured = config.for_profile(profile).ok_or_else(|| {
                AppError::ConfigError(format!("未知的 LLM max_tokens 场景配置: {profile}"))
            })?;
            let resolved = request
                .max_tokens
                .map_or(configured, |minimum| configured.max(minimum));
            (resolved, format!("profile={profile}"))
        }
        None => match request.max_tokens {
            Some(max_tokens) => (max_tokens, "request.max_tokens".to_string()),
            None => (max_output_tokens, "model.max_output_tokens".to_string()),
        },
    };
    ensure_max_tokens_within_model_cap(
        Some(output_token_reserve),
        request.max_tokens_profile.as_deref(),
        max_output_tokens,
        model.model.as_str(),
    )?;

    Ok(EffectiveLlmPreviewBudget {
        model_name: model.model,
        max_context_tokens,
        max_output_tokens,
        output_token_reserve,
        output_token_source,
    })
}

async fn resolve_preview_llm_model(db: &DatabaseConnection) -> Result<llm_config::Model, AppError> {
    if let Some(model) = runtime_config::current_llm_model() {
        return Ok(model);
    }
    if runtime_config::uses_system_default_llm_template() {
        let mut model = crate::global_ai_config::resolve_llm_config(db, None).await?;
        model.api_key_encrypted = None;
        return Ok(model);
    }
    if runtime_config::current_user_id().is_some() {
        return Err(AppError::ConfigError(
            "LLM 请求预览需要用户 LLM 配置快照或系统默认模板；请先在用户配置页选择默认 LLM"
                .to_string(),
        ));
    }
    crate::global_ai_config::resolve_llm_config(db, None).await
}

fn ensure_max_tokens_within_model_cap(
    max_tokens: Option<usize>,
    profile: Option<&str>,
    model_max_output_tokens: usize,
    model_name: &str,
) -> Result<(), AppError> {
    let Some(max_tokens) = max_tokens else {
        return Ok(());
    };
    if max_tokens <= model_max_output_tokens {
        return Ok(());
    }
    let source = profile
        .map(|profile| format!("profile={profile}"))
        .unwrap_or_else(|| "显式 max_tokens".to_string());
    Err(AppError::ConfigError(format!(
        "LLM 请求输出上限超过模型能力：model={}, {}, max_tokens={}, model_max_output_tokens={}",
        model_name, source, max_tokens, model_max_output_tokens
    )))
}

struct QueueTrackedLlmStream {
    inner: Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>,
    db: Arc<DatabaseConnection>,
    task_queue: Arc<PersistentTaskQueue>,
    permit: Option<TaskPermit>,
    task_id: String,
    trace_context: Option<LlmTraceContext>,
    model_name: String,
    started: Instant,
    usage: Option<UsageInfo>,
    finish_reason: Option<String>,
}

impl QueueTrackedLlmStream {
    fn new(
        inner: Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>,
        db: Arc<DatabaseConnection>,
        task_queue: Arc<PersistentTaskQueue>,
        permit: TaskPermit,
        trace_context: Option<LlmTraceContext>,
        model_name: String,
        started: Instant,
    ) -> Self {
        let task_id = permit.task_id().to_string();
        Self {
            inner,
            db,
            task_queue,
            permit: Some(permit),
            task_id,
            trace_context,
            model_name,
            started,
            usage: None,
            finish_reason: None,
        }
    }

    fn finish_success(&mut self) {
        let Some(permit) = self.permit.take() else {
            return;
        };
        let task_id = self.task_id.clone();
        let task_queue = self.task_queue.clone();
        let db = self.db.clone();
        let trace_context = self.trace_context.clone();
        let model_name = self.model_name.clone();
        let duration_ms = Some(self.started.elapsed().as_millis() as i64);
        let usage = self.usage.clone();
        let finish_reason = self.finish_reason.clone();
        permit.complete();
        tokio::spawn(async move {
            if let Err(err) = task_queue.mark_done(&task_id).await {
                tracing::warn!(
                    task_id = %task_id,
                    error = %err,
                    "同步 LLM stream 队列完成状态失败"
                );
            }
            record_dynamic_llm_trace(
                db.as_ref(),
                trace_context.as_ref(),
                Some(model_name.as_str()),
                usage,
                duration_ms,
                true,
                "completed",
                None,
                None,
                finish_reason.as_deref(),
            )
            .await;
        });
    }

    fn finish_failed(&mut self, message: String) {
        let Some(permit) = self.permit.take() else {
            return;
        };
        let task_id = self.task_id.clone();
        let task_queue = self.task_queue.clone();
        let db = self.db.clone();
        let trace_context = self.trace_context.clone();
        let model_name = self.model_name.clone();
        let duration_ms = Some(self.started.elapsed().as_millis() as i64);
        permit.fail(message.clone());
        tokio::spawn(async move {
            if let Err(err) = task_queue.mark_failed(&task_id, &message).await {
                tracing::warn!(
                    task_id = %task_id,
                    error = %err,
                    "同步 LLM stream 队列失败状态失败"
                );
            }
            record_dynamic_llm_trace(
                db.as_ref(),
                trace_context.as_ref(),
                Some(model_name.as_str()),
                None,
                duration_ms,
                true,
                "failed",
                Some("stream_error"),
                Some(message.as_str()),
                None,
            )
            .await;
        });
    }
}

impl Stream for QueueTrackedLlmStream {
    type Item = Result<ChatChunk, AppError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                if let Some(usage) = chunk.usage.clone() {
                    self.usage = Some(usage);
                }
                if let Some(finish_reason) = chunk.finish_reason.clone() {
                    self.finish_reason = Some(finish_reason);
                }
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(err))) => {
                self.finish_failed(err.to_string());
                Poll::Ready(Some(Err(err)))
            }
            Poll::Ready(None) => {
                self.finish_success();
                Poll::Ready(None)
            }
            other => other,
        }
    }
}

impl Unpin for QueueTrackedLlmStream {}

impl Drop for QueueTrackedLlmStream {
    fn drop(&mut self) {
        if self.permit.is_some() {
            self.finish_failed("LLM stream 在完成前被释放".to_string());
        }
    }
}

pub async fn load_llm_max_tokens_config(
    db: &DatabaseConnection,
) -> Result<LlmMaxTokensConfig, AppError> {
    let Some(setting) = system_settings::Entity::find()
        .filter(system_settings::Column::Key.eq(LLM_MAX_TOKENS_SETTING_KEY))
        .one(db)
        .await
        .map_err(AppError::Database)?
    else {
        return Ok(LlmMaxTokensConfig::default());
    };

    let config: LlmMaxTokensConfig = serde_json::from_value(setting.value)
        .map_err(|err| AppError::ConfigError(format!("LLM max_tokens 配置解析失败: {err}")))?;
    config
        .validate()
        .map_err(|err| AppError::ConfigError(format!("LLM max_tokens 配置无效: {err}")))?;
    Ok(config)
}

async fn record_dynamic_llm_trace(
    db: &DatabaseConnection,
    trace_context: Option<&LlmTraceContext>,
    model: Option<&str>,
    usage: Option<a4n_engine::llm::provider::UsageInfo>,
    duration_ms: Option<i64>,
    streamed: bool,
    status: &str,
    error_kind: Option<&str>,
    error_message: Option<&str>,
    finish_reason: Option<&str>,
) {
    let Some(trace_context) = trace_context else {
        return;
    };
    let result = record_llm_call_trace(
        db,
        RecordLlmCallTraceInput {
            generation_run_id: trace_context.generation_run_id,
            workflow_run_id: trace_context.workflow_run_id,
            call_key: trace_context.call_key.clone(),
            input_fingerprint_sha256: trace_context.input_fingerprint_sha256.clone(),
            project_id: trace_context.project_id,
            root_project_id: trace_context.root_project_id,
            workflow: trace_context.workflow.clone(),
            step_key: trace_context.step_key.clone(),
            provider: None,
            model: model.map(ToString::to_string),
            context_plan_json: trace_context.context_plan_json.clone(),
            context_pack_refs_json: trace_context.context_pack_refs_json.clone(),
            usage,
            duration_ms,
            streamed,
            status: status.to_string(),
            error_kind: error_kind.map(ToString::to_string),
            error_message: error_message.map(ToString::to_string),
            finish_reason: finish_reason.map(ToString::to_string),
        },
    )
    .await;
    if let Err(err) = result {
        tracing::warn!(
            target: "a4n::llm::trace",
            workflow = %trace_context.workflow,
            step_key = %trace_context.step_key,
            error = %err,
            "记录 LLM 调用 trace 失败"
        );
    }
}

fn llm_error_kind(error: &AppError) -> &'static str {
    match error {
        AppError::Internal(_) => "internal",
        AppError::Database(_) => "database",
        AppError::NotFound(_) => "not_found",
        AppError::Forbidden(_) => "forbidden",
        AppError::InvalidArgument(_) => "invalid_argument",
        AppError::InvalidState { .. } => "invalid_state",
        AppError::LlmError(_) => "llm_error",
        AppError::VectorStoreError(_) => "vector_store_error",
        AppError::EmbeddingError(_) => "embedding_error",
        AppError::Cancelled => "cancelled",
        AppError::Timeout(_) => "timeout",
        AppError::ConfigError(_) => "config_error",
        AppError::Io(_) => "io",
        AppError::Serialization(_) => "serialization",
    }
}

fn llm_trace_status_for_error(error: &AppError) -> &'static str {
    match error {
        AppError::Cancelled => "cancelled",
        _ => "failed",
    }
}

async fn prepare_dynamic_llm_trace_context(
    db: &DatabaseConnection,
    provider: &dyn LlmProvider,
    request: &mut ChatRequest,
) -> Result<Option<LlmTraceContext>, AppError> {
    let Some(mut trace_context) = request.trace_context.clone() else {
        return Ok(None);
    };

    let caps = provider.capabilities();
    let output_reserve = request.max_tokens.unwrap_or(caps.max_output_tokens);
    let system_messages = system_messages(&request.messages);
    let system_tokens = provider.count_messages_tokens(&system_messages);
    let mut system_requirements = Vec::new();

    if !system_messages.is_empty() {
        let system_ref =
            build_system_prefix_pack_ref(db, &trace_context, &system_messages, system_tokens)
                .await?;
        system_requirements.push(LlmContextPackRequirement {
            pack_kind: system_ref.pack_kind.clone(),
            required: true,
            stable_prefix: true,
            variants: vec![system_ref],
        });
    }

    let plan = match request.context_manifest_json.clone() {
        Some(manifest_json) => {
            let manifest: LlmContextManifest =
                serde_json::from_value(manifest_json).map_err(|err| {
                    AppError::InvalidArgument(format!("LLM 上下文 manifest 解析失败: {err}"))
                })?;
            validate_context_manifest(&manifest)?;
            let (mut requirements, content_by_ref) =
                build_manifest_requirements(db, provider, &trace_context, &manifest).await?;
            let mut all_requirements = system_requirements;
            all_requirements.append(&mut requirements);
            let dynamic_delta_tokens =
                provider.count_tokens(&render_context_manifest_user_content(&manifest, &[])?);
            let mut plan = ContextBudgetPlanner::default().plan_requirements(
                caps.max_context_tokens,
                output_reserve,
                all_requirements,
                dynamic_delta_tokens,
            )?;
            hydrate_selected_context_pack_content(&mut plan.selected_packs, &content_by_ref);
            replace_request_user_content_from_manifest(request, &manifest, &plan.selected_packs)?;
            plan
        }
        None => {
            let non_system_messages = non_system_messages(&request.messages);
            let task_delta_tokens = provider.count_messages_tokens(&non_system_messages);
            ContextBudgetPlanner::default().plan_requirements(
                caps.max_context_tokens,
                output_reserve,
                system_requirements,
                task_delta_tokens,
            )?
        }
    };
    trace_context.context_plan_json =
        Some(serde_json::to_value(&plan).map_err(AppError::Serialization)?);
    trace_context.context_pack_refs_json =
        Some(serde_json::to_value(&plan.selected_pack_refs).map_err(AppError::Serialization)?);
    request.trace_context = Some(trace_context.clone());
    Ok(Some(trace_context))
}

type ContextContentByRef = HashMap<(String, String, String), String>;

async fn build_manifest_requirements(
    db: &DatabaseConnection,
    provider: &dyn LlmProvider,
    trace_context: &LlmTraceContext,
    manifest: &LlmContextManifest,
) -> Result<(Vec<LlmContextPackRequirement>, ContextContentByRef), AppError> {
    let project_id = trace_context.project_id.ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "LLM trace 缺少 project_id，无法记录上下文 manifest：workflow={}, step={}",
            trace_context.workflow, trace_context.step_key
        ))
    })?;
    let root_project_id = trace_context.root_project_id.unwrap_or(project_id);
    let mut requirements = Vec::new();
    let mut content_by_ref = HashMap::new();

    for pack in &manifest.packs {
        let mut variants = Vec::new();
        for variant in &pack.variants {
            let draft = LlmContextPackDraft::from_json(LlmContextPackDraftInput {
                project_id,
                root_project_id,
                scope_kind: pack.scope_kind.clone(),
                workflow: trace_context.workflow.clone(),
                pack_kind: pack.pack_kind.clone(),
                source_refs_json: pack.source_refs_json.clone(),
                config_hash: pack.config_hash.clone(),
                compression_level: variant.compression_level.clone(),
                canonical_content_json: variant.canonical_content_json.clone(),
            })?;
            let canonical_content_text = draft.canonical_content_text.clone();
            let model = record_or_reuse_context_pack(db, draft).await?;
            let mut pack_ref = context_pack_model_ref(&model);
            pack_ref.token_estimate = provider.count_tokens(&canonical_content_text);
            content_by_ref.insert(selected_pack_key(&pack_ref), canonical_content_text);
            variants.push(pack_ref);
        }
        requirements.push(LlmContextPackRequirement {
            pack_kind: pack.pack_kind.clone(),
            required: pack.required,
            stable_prefix: pack.stable_prefix,
            variants,
        });
    }

    Ok((requirements, content_by_ref))
}

fn hydrate_selected_context_pack_content(
    selected_packs: &mut [PlannedContextPack],
    content_by_ref: &ContextContentByRef,
) {
    for selected in selected_packs {
        selected.canonical_content_text = content_by_ref
            .get(&selected_pack_key(&selected.pack_ref))
            .cloned();
    }
}

fn selected_pack_key(pack_ref: &LlmContextPackRef) -> (String, String, String) {
    (
        pack_ref.pack_kind.clone(),
        pack_ref.compression_level.clone(),
        pack_ref.content_sha256.clone(),
    )
}

fn system_messages(messages: &[ChatMessage]) -> Vec<ChatMessage> {
    messages
        .iter()
        .filter(|message| matches!(&message.role, MessageRole::System))
        .cloned()
        .collect()
}

fn non_system_messages(messages: &[ChatMessage]) -> Vec<ChatMessage> {
    messages
        .iter()
        .filter(|message| !matches!(&message.role, MessageRole::System))
        .cloned()
        .collect()
}

async fn build_system_prefix_pack_ref(
    db: &DatabaseConnection,
    trace_context: &LlmTraceContext,
    system_messages: &[ChatMessage],
    token_estimate: usize,
) -> Result<LlmContextPackRef, AppError> {
    let content = json!({
        "schema_version": 1,
        "messages": system_messages,
    });
    let project_id = trace_context.project_id.ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "LLM trace 缺少 project_id，无法记录上下文包：workflow={}, step={}",
            trace_context.workflow, trace_context.step_key
        ))
    })?;
    let root_project_id = trace_context.root_project_id.unwrap_or(project_id);
    let draft = LlmContextPackDraft::from_json(LlmContextPackDraftInput {
        project_id,
        root_project_id,
        scope_kind: "llm_request".to_string(),
        workflow: trace_context.workflow.clone(),
        pack_kind: "system_prompt_prefix".to_string(),
        source_refs_json: Some(json!({
            "source": "chat_request.system_messages",
            "step_key": trace_context.step_key.clone(),
        })),
        config_hash: "system_prompt_prefix:v1".to_string(),
        compression_level: COMPRESSION_FULL_RAW.to_string(),
        canonical_content_json: content,
    })?;
    let model = record_or_reuse_context_pack(db, draft).await?;
    let mut pack_ref = context_pack_model_ref(&model);
    pack_ref.token_estimate = token_estimate;
    Ok(pack_ref)
}

fn make_provider(m: &llm_config::Model) -> Result<HttpLlmProvider, AppError> {
    let timeout_secs = u64::try_from(m.timeout_secs).map_err(|_| {
        AppError::ConfigError(format!("LLM 配置 {} 的 timeout_secs 必须大于 0", m.name))
    })?;
    if timeout_secs == 0 {
        return Err(AppError::ConfigError(format!(
            "LLM 配置 {} 的 timeout_secs 必须大于 0",
            m.name
        )));
    }

    let max_context_tokens = usize::try_from(m.max_context_tokens).map_err(|_| {
        AppError::ConfigError(format!(
            "LLM 配置 {} 的 max_context_tokens 必须大于 0",
            m.name
        ))
    })?;
    let max_output_tokens = usize::try_from(m.max_output_tokens).map_err(|_| {
        AppError::ConfigError(format!(
            "LLM 配置 {} 的 max_output_tokens 必须大于 0",
            m.name
        ))
    })?;
    if max_context_tokens == 0 || max_output_tokens == 0 || max_output_tokens >= max_context_tokens
    {
        return Err(AppError::ConfigError(format!(
            "LLM 配置 {} 的模型能力无效：max_context_tokens={}, max_output_tokens={}",
            m.name, max_context_tokens, max_output_tokens
        )));
    }

    HttpLlmProvider::new_with_timeout_options_and_capabilities(
        m.base_url.clone(),
        m.model.clone(),
        m.api_key_encrypted.clone(),
        timeout_secs,
        m.request_options_json.clone(),
        ModelCapabilities {
            function_calling: true,
            vision: true,
            max_context_tokens,
            max_output_tokens,
        },
    )
}

#[async_trait]
impl LlmProvider for DynamicLlmProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
        self.chat_with_config_id(None, request).await
    }

    async fn chat_stream(
        &self,
        mut request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        let profile = request.max_tokens_profile.clone();
        let temperature = request.temperature;
        let message_count = request.messages.len();
        let prompt_chars: usize = request.messages.iter().map(|msg| msg.content.len()).sum();
        let provider = self.resolve_default().await?;
        let model_name = provider.model_name().to_string();
        let model_max_output_tokens = provider.capabilities().max_output_tokens;
        request.max_tokens = self.resolve_max_tokens(&request).await?;
        ensure_max_tokens_within_model_cap(
            request.max_tokens,
            profile.as_deref(),
            model_max_output_tokens,
            model_name.as_str(),
        )?;
        let trace_context =
            prepare_dynamic_llm_trace_context(self.db.as_ref(), &provider, &mut request).await?;
        if trace_context.is_some() {
            request.stream_include_usage = true;
        }
        tracing::debug!(
            target: "a4n::llm",
            model = %model_name,
            profile = ?profile,
            max_tokens = ?request.max_tokens,
            temperature = ?temperature,
            message_count,
            prompt_chars,
            "LLM stream 请求"
        );
        let started = Instant::now();
        let submitted_id = self
            .task_queue
            .submit(TaskType::LlmCall, TaskPriority::Medium, None)
            .await?;
        let mut submitted_guard = self.task_queue.submitted_task_guard(submitted_id.clone());
        let Some(permit) = self
            .task_queue
            .acquire_task_with_cancel(&submitted_id, tokio_util::sync::CancellationToken::new())
            .await?
        else {
            if let Err(err) = self.task_queue.cancel(&submitted_id).await {
                tracing::warn!(
                    target: "a4n::llm",
                    task_id = %submitted_id,
                    error = %err,
                    "同步取消 LLM stream 队列任务失败"
                );
            }
            submitted_guard.disarm();
            return Err(AppError::Cancelled);
        };
        submitted_guard.disarm();
        let running_id = permit.task_id().to_string();
        let result = provider.chat_stream(request).await;
        match result {
            Ok(stream) => Ok(Box::pin(QueueTrackedLlmStream::new(
                stream,
                self.db.clone(),
                self.task_queue.clone(),
                permit,
                trace_context,
                model_name,
                started,
            ))),
            Err(error) => {
                let message = error.to_string();
                permit.fail(message.clone());
                if let Err(status_err) = self.task_queue.mark_failed(&running_id, &message).await {
                    tracing::warn!(
                        target: "a4n::llm",
                        task_id = %running_id,
                        error = %status_err,
                        "同步 LLM stream 建立失败队列状态失败"
                    );
                }
                record_dynamic_llm_trace(
                    self.db.as_ref(),
                    trace_context.as_ref(),
                    Some(model_name.as_str()),
                    None,
                    Some(started.elapsed().as_millis() as i64),
                    true,
                    llm_trace_status_for_error(&error),
                    Some(llm_error_kind(&error)),
                    Some(message.as_str()),
                    None,
                )
                .await;
                tracing::warn!(
                    target: "a4n::llm",
                    model = %model_name,
                    profile = ?profile,
                    duration_ms = started.elapsed().as_millis(),
                    error = %error,
                    "LLM stream 建立失败"
                );
                Err(error)
            }
        }
    }

    async fn test_connection(&self) -> Result<bool, AppError> {
        let provider = self.resolve_default().await?;
        self.task_queue
            .run_tracked(TaskType::LlmCall, TaskPriority::High, None, || async move {
                provider.test_connection().await
            })
            .await
    }

    fn model_name(&self) -> &str {
        "dynamic-default-llm"
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            function_calling: true,
            vision: true,
            max_context_tokens: DEFAULT_LLM_MAX_CONTEXT_TOKENS,
            max_output_tokens: DEFAULT_LLM_MAX_OUTPUT_TOKENS,
        }
    }

    fn count_tokens(&self, text: &str) -> usize {
        let chinese_chars = text.chars().filter(|c| u32::from(*c) > 0x2E80).count();
        let other_words = if chinese_chars == text.len() {
            0
        } else {
            text.split_whitespace().count()
        };
        ((chinese_chars as f32 * 0.5) + (other_words as f32 * 1.3)).ceil() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::ensure_max_tokens_within_model_cap;
    use a4n_core::config::app_config::DEFAULT_LLM_MAX_OUTPUT_TOKENS;
    use a4n_core::error::AppError;

    #[test]
    fn max_tokens_cap_accepts_missing_or_within_limit() {
        assert!(
            ensure_max_tokens_within_model_cap(
                None,
                None,
                DEFAULT_LLM_MAX_OUTPUT_TOKENS,
                "deepseek-chat"
            )
            .is_ok()
        );
        assert!(
            ensure_max_tokens_within_model_cap(
                Some(DEFAULT_LLM_MAX_OUTPUT_TOKENS),
                Some("media_prompt"),
                DEFAULT_LLM_MAX_OUTPUT_TOKENS,
                "deepseek-chat",
            )
            .is_ok()
        );
    }

    #[test]
    fn max_tokens_cap_rejects_profile_above_model_limit() {
        let err = ensure_max_tokens_within_model_cap(
            Some(12_288),
            Some("character_sync"),
            DEFAULT_LLM_MAX_OUTPUT_TOKENS,
            "deepseek-chat",
        )
        .expect_err("profile output limit must not exceed model capability");

        assert!(matches!(err, AppError::ConfigError(_)));
        assert!(err.to_string().contains("profile=character_sync"));
        assert!(err.to_string().contains("model_max_output_tokens=8192"));
    }
}
