//! LLM 上下文包、预算规划和上下文感知请求构造。

use std::collections::BTreeSet;

use a4n_core::error::AppError;
use a4n_core::models::generation::llm_context_pack;
use a4n_engine::llm::provider::{ChatMessage, ChatRequest, MessageRole};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::source::source_content_sha256;

pub const COMPRESSION_FULL_RAW: &str = "full_raw";
pub const COMPRESSION_FOCUSED_RAW: &str = "focused_raw";
pub const COMPRESSION_COMPRESSED_SCENE: &str = "compressed_scene";
pub const COMPRESSION_MINIMAL: &str = "minimal";

const DEFAULT_CONTEXT_SAFETY_MARGIN_TOKENS: usize = 2048;
const DEFAULT_CONTEXT_INPUT_UTILIZATION_PERCENT: u8 = 85;
pub const CONTEXT_MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Default, Clone)]
pub struct LlmContextLedger {
    packs: Vec<LlmContextManifestPack>,
}

#[derive(Debug, Clone)]
pub struct LlmContextLedgerPackDraft {
    pub compression_level: String,
    pub canonical_content_json: Value,
}

#[derive(Debug, Clone)]
pub struct LlmContextLedgerPackInput {
    pub scope_kind: String,
    pub pack_kind: String,
    pub source_refs_json: Option<Value>,
    pub config_identity_json: Value,
    pub required: bool,
    pub stable_prefix: bool,
    pub variants: Vec<LlmContextLedgerPackDraft>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmContextPackDraft {
    pub project_id: i64,
    pub root_project_id: i64,
    pub scope_kind: String,
    pub workflow: String,
    pub pack_kind: String,
    pub source_refs_json: Option<Value>,
    pub config_hash: String,
    pub canonical_content_json: Value,
    pub canonical_content_text: String,
    pub content_sha256: String,
    pub compression_level: String,
    pub token_estimate: usize,
}

#[derive(Debug, Clone)]
pub struct LlmContextPackDraftInput {
    pub project_id: i64,
    pub root_project_id: i64,
    pub scope_kind: String,
    pub workflow: String,
    pub pack_kind: String,
    pub source_refs_json: Option<Value>,
    pub config_hash: String,
    pub compression_level: String,
    pub canonical_content_json: Value,
}

impl LlmContextLedger {
    pub fn new() -> Self {
        Self { packs: Vec::new() }
    }

    pub fn add_pack(&mut self, input: LlmContextLedgerPackInput) -> Result<(), AppError> {
        let pack_kind = input.pack_kind;
        if input.variants.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "LLM 上下文包 {pack_kind} 至少需要一个压缩变体"
            )));
        }
        let mut levels = BTreeSet::new();
        let variants = input
            .variants
            .into_iter()
            .map(|variant| {
                if variant.compression_level.trim().is_empty() {
                    return Err(AppError::InvalidArgument(format!(
                        "LLM 上下文包 {pack_kind} 存在空 compression_level"
                    )));
                }
                if !levels.insert(variant.compression_level.clone()) {
                    return Err(AppError::InvalidArgument(format!(
                        "LLM 上下文包 {pack_kind} 存在重复 compression_level：{}",
                        variant.compression_level
                    )));
                }
                Ok(LlmContextManifestPackVariant {
                    compression_level: variant.compression_level,
                    canonical_content_json: variant.canonical_content_json,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        let config_hash = context_config_hash(&pack_kind, &input.config_identity_json)?;
        self.packs.push(LlmContextManifestPack {
            scope_kind: input.scope_kind,
            pack_kind,
            source_refs_json: input.source_refs_json,
            config_hash,
            required: input.required,
            stable_prefix: input.stable_prefix,
            variants,
        });
        Ok(())
    }

    pub fn into_manifest(
        self,
        dynamic_user_content: impl Into<String>,
        output_contract: Option<String>,
    ) -> LlmContextManifest {
        LlmContextManifest {
            schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION,
            packs: self.packs,
            dynamic_user_content: dynamic_user_content.into(),
            output_contract,
        }
    }
}

impl LlmContextLedgerPackDraft {
    pub fn new(compression_level: impl Into<String>, canonical_content_json: Value) -> Self {
        Self {
            compression_level: compression_level.into(),
            canonical_content_json,
        }
    }
}

pub fn context_config_hash(
    pack_kind: &str,
    config_identity_json: &Value,
) -> Result<String, AppError> {
    let canonical_identity = canonical_json_text(&serde_json::json!({
        "schema_version": CONTEXT_MANIFEST_SCHEMA_VERSION,
        "pack_kind": pack_kind,
        "config_identity": config_identity_json
    }))?;
    Ok(source_content_sha256(&canonical_identity))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmContextPackRef {
    pub pack_kind: String,
    pub content_sha256: String,
    pub compression_level: String,
    pub token_estimate: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmContextPackRequirement {
    pub pack_kind: String,
    pub required: bool,
    pub stable_prefix: bool,
    pub variants: Vec<LlmContextPackRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmContextManifest {
    pub schema_version: u32,
    pub packs: Vec<LlmContextManifestPack>,
    pub dynamic_user_content: String,
    pub output_contract: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmContextManifestPack {
    pub scope_kind: String,
    pub pack_kind: String,
    pub source_refs_json: Option<Value>,
    pub config_hash: String,
    pub required: bool,
    pub stable_prefix: bool,
    pub variants: Vec<LlmContextManifestPackVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmContextManifestPackVariant {
    pub compression_level: String,
    pub canonical_content_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlannedContextPack {
    pub pack_ref: LlmContextPackRef,
    pub stable_prefix: bool,
    #[serde(skip)]
    pub canonical_content_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmContextRequestPreview {
    pub manifest_json: Value,
    pub plan_json: Value,
    pub pack_refs_json: Value,
    pub stable_prefix_tokens: usize,
    pub dynamic_delta_tokens: usize,
    pub available_input_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmContextPlan {
    pub model_context_tokens: usize,
    pub output_token_reserve: usize,
    pub safety_margin_tokens: usize,
    pub input_utilization_percent: u8,
    pub raw_available_input_tokens: usize,
    pub available_input_tokens: usize,
    pub stable_prefix_tokens: usize,
    pub dynamic_delta_tokens: usize,
    pub selected_compression_level: String,
    pub selected_pack_refs: Vec<LlmContextPackRef>,
    #[serde(skip)]
    pub selected_packs: Vec<PlannedContextPack>,
    pub dropped_optional_pack_refs: Vec<LlmContextPackRef>,
    pub estimated_input_tokens: usize,
}

#[derive(Debug, Clone)]
pub struct ContextBudgetPlanner {
    safety_margin_tokens: usize,
    input_utilization_percent: u8,
}

impl LlmContextPackDraft {
    pub fn from_json(input: LlmContextPackDraftInput) -> Result<Self, AppError> {
        let canonical_content_text = canonical_json_text(&input.canonical_content_json)?;
        let token_estimate = estimate_context_tokens(&canonical_content_text);
        let content_sha256 = source_content_sha256(&canonical_content_text);
        Ok(Self {
            project_id: input.project_id,
            root_project_id: input.root_project_id,
            scope_kind: input.scope_kind,
            workflow: input.workflow,
            pack_kind: input.pack_kind,
            source_refs_json: input.source_refs_json,
            config_hash: input.config_hash,
            canonical_content_json: input.canonical_content_json,
            canonical_content_text,
            content_sha256,
            compression_level: input.compression_level,
            token_estimate,
        })
    }
}

pub async fn record_or_reuse_context_pack<C>(
    db: &C,
    draft: LlmContextPackDraft,
) -> Result<llm_context_pack::Model, AppError>
where
    C: ConnectionTrait,
{
    if let Some(existing) = llm_context_pack::Entity::find()
        .filter(llm_context_pack::Column::ProjectId.eq(draft.project_id))
        .filter(llm_context_pack::Column::Workflow.eq(draft.workflow.as_str()))
        .filter(llm_context_pack::Column::PackKind.eq(draft.pack_kind.as_str()))
        .filter(llm_context_pack::Column::CompressionLevel.eq(draft.compression_level.as_str()))
        .filter(llm_context_pack::Column::ContentSha256.eq(draft.content_sha256.as_str()))
        .filter(llm_context_pack::Column::ConfigHash.eq(draft.config_hash.as_str()))
        .one(db)
        .await
        .map_err(AppError::Database)?
    {
        return Ok(existing);
    }

    let now = Utc::now().naive_utc();
    llm_context_pack::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        project_id: Set(draft.project_id),
        root_project_id: Set(draft.root_project_id),
        scope_kind: Set(draft.scope_kind),
        workflow: Set(draft.workflow),
        pack_kind: Set(draft.pack_kind),
        source_refs_json: Set(draft.source_refs_json),
        config_hash: Set(draft.config_hash),
        content_sha256: Set(draft.content_sha256),
        canonical_content_json: Set(draft.canonical_content_json),
        canonical_content_text: Set(draft.canonical_content_text),
        compression_level: Set(draft.compression_level),
        token_estimate: Set(usize_to_i32(draft.token_estimate, "token_estimate")?),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

pub fn context_pack_model_ref(model: &llm_context_pack::Model) -> LlmContextPackRef {
    LlmContextPackRef {
        pack_kind: model.pack_kind.clone(),
        content_sha256: model.content_sha256.clone(),
        compression_level: model.compression_level.clone(),
        token_estimate: model.token_estimate.max(0) as usize,
    }
}

impl Default for ContextBudgetPlanner {
    fn default() -> Self {
        Self {
            safety_margin_tokens: DEFAULT_CONTEXT_SAFETY_MARGIN_TOKENS,
            input_utilization_percent: DEFAULT_CONTEXT_INPUT_UTILIZATION_PERCENT,
        }
    }
}

impl ContextBudgetPlanner {
    pub fn new(safety_margin_tokens: usize) -> Self {
        Self {
            safety_margin_tokens,
            input_utilization_percent: DEFAULT_CONTEXT_INPUT_UTILIZATION_PERCENT,
        }
    }

    pub fn with_input_utilization_percent(mut self, input_utilization_percent: u8) -> Self {
        self.input_utilization_percent = input_utilization_percent;
        self
    }

    pub fn plan_requirements(
        &self,
        model_context_tokens: usize,
        output_token_reserve: usize,
        requirements: Vec<LlmContextPackRequirement>,
        dynamic_delta_tokens: usize,
    ) -> Result<LlmContextPlan, AppError> {
        if self.input_utilization_percent == 0 || self.input_utilization_percent > 100 {
            return Err(AppError::InvalidArgument(format!(
                "LLM 上下文输入利用率必须在 1..=100，当前为 {}",
                self.input_utilization_percent
            )));
        }
        if model_context_tokens <= output_token_reserve + self.safety_margin_tokens {
            return Err(AppError::InvalidArgument(format!(
                "模型上下文窗口不足：context={model_context_tokens}, output_reserve={output_token_reserve}, safety_margin={}",
                self.safety_margin_tokens
            )));
        }
        let raw_available_input_tokens =
            model_context_tokens - output_token_reserve - self.safety_margin_tokens;
        let available_input_tokens = raw_available_input_tokens
            .saturating_mul(self.input_utilization_percent as usize)
            / 100;
        if dynamic_delta_tokens > available_input_tokens {
            return Err(AppError::InvalidArgument(format!(
                "当前任务动态输入约 {dynamic_delta_tokens} tokens，超过模型可用输入窗口 {available_input_tokens}"
            )));
        }

        let mut selected_refs = Vec::new();
        let mut selected_packs = Vec::new();
        let mut dropped = Vec::new();
        let mut stable_prefix_tokens = 0_usize;
        let mut used = dynamic_delta_tokens;
        let remaining_required_min_tokens = remaining_required_min_token_suffix(&requirements);

        for requirement_index in 0..requirements.len() {
            let requirement = &requirements[requirement_index];
            if requirement.variants.is_empty() {
                if requirement.required {
                    return Err(AppError::InvalidArgument(format!(
                        "必需上下文包 {} 没有可用压缩变体",
                        requirement.pack_kind
                    )));
                }
                continue;
            }

            let later_required_min_tokens = remaining_required_min_tokens[requirement_index + 1];

            let mut variants = requirement.variants.clone();
            variants.sort_by(|left, right| {
                compression_rank(right.compression_level.as_str())
                    .cmp(&compression_rank(left.compression_level.as_str()))
                    .then_with(|| left.token_estimate.cmp(&right.token_estimate))
            });

            let selected_variant = variants
                .iter()
                .find(|variant| {
                    used.saturating_add(variant.token_estimate)
                        .saturating_add(later_required_min_tokens)
                        <= available_input_tokens
                })
                .cloned();

            match selected_variant {
                Some(pack_ref) => {
                    used = used.saturating_add(pack_ref.token_estimate);
                    if requirement.stable_prefix {
                        stable_prefix_tokens =
                            stable_prefix_tokens.saturating_add(pack_ref.token_estimate);
                    }
                    selected_refs.push(pack_ref.clone());
                    selected_packs.push(PlannedContextPack {
                        pack_ref,
                        stable_prefix: requirement.stable_prefix,
                        canonical_content_text: None,
                    });
                }
                None if requirement.required => {
                    let smallest = variants
                        .iter()
                        .min_by_key(|variant| variant.token_estimate)
                        .expect("variants checked non-empty");
                    return Err(AppError::InvalidArgument(format!(
                        "必需上下文包 {} 的最小压缩变体约 {} tokens，加入后超过模型可用输入窗口 {}",
                        requirement.pack_kind, smallest.token_estimate, available_input_tokens
                    )));
                }
                None => {
                    dropped.extend(variants);
                }
            }
        }

        let selected_compression_level = selected_refs
            .iter()
            .map(|pack| pack.compression_level.as_str())
            .max_by_key(|level| compression_rank(level))
            .unwrap_or(COMPRESSION_MINIMAL)
            .to_string();

        Ok(LlmContextPlan {
            model_context_tokens,
            output_token_reserve,
            safety_margin_tokens: self.safety_margin_tokens,
            input_utilization_percent: self.input_utilization_percent,
            raw_available_input_tokens,
            available_input_tokens,
            stable_prefix_tokens,
            dynamic_delta_tokens,
            selected_compression_level,
            selected_pack_refs: selected_refs,
            selected_packs,
            dropped_optional_pack_refs: dropped,
            estimated_input_tokens: used,
        })
    }
}

fn remaining_required_min_token_suffix(requirements: &[LlmContextPackRequirement]) -> Vec<usize> {
    let mut suffix = vec![0_usize; requirements.len() + 1];
    for index in (0..requirements.len()).rev() {
        let current_min = if requirements[index].required {
            requirements[index]
                .variants
                .iter()
                .map(|variant| variant.token_estimate)
                .min()
                .unwrap_or(0)
        } else {
            0
        };
        suffix[index] = suffix[index + 1].saturating_add(current_min);
    }
    suffix
}

pub fn context_aware_system_user_request(
    system_prompt: impl Into<String>,
    manifest: LlmContextManifest,
    temperature: Option<f32>,
    max_tokens: Option<usize>,
    max_tokens_profile: Option<String>,
) -> Result<ChatRequest, AppError> {
    validate_context_manifest(&manifest)?;
    let preview_user_content = render_context_manifest_user_content(&manifest, &[])?;
    Ok(ChatRequest::system_user(
        system_prompt,
        preview_user_content,
        temperature,
        max_tokens,
        max_tokens_profile,
    )
    .with_context_manifest_json(serde_json::to_value(&manifest).map_err(AppError::Serialization)?))
}

pub fn validate_context_manifest(manifest: &LlmContextManifest) -> Result<(), AppError> {
    if manifest.schema_version != CONTEXT_MANIFEST_SCHEMA_VERSION {
        return Err(AppError::InvalidArgument(format!(
            "不支持的 LLM 上下文 manifest 版本：{}",
            manifest.schema_version
        )));
    }
    if manifest.packs.is_empty() {
        return Err(AppError::InvalidArgument(
            "LLM 上下文 manifest 至少需要一个上下文包".to_string(),
        ));
    }
    let mut pack_kinds = BTreeSet::new();
    for pack in &manifest.packs {
        if pack.pack_kind.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "LLM 上下文包 pack_kind 不能为空".to_string(),
            ));
        }
        if !pack_kinds.insert(pack.pack_kind.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "LLM 上下文 manifest 存在重复 pack_kind：{}",
                pack.pack_kind
            )));
        }
        if pack.config_hash.trim().is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "LLM 上下文包 {} 缺少 config_hash",
                pack.pack_kind
            )));
        }
        if pack.variants.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "LLM 上下文包 {} 至少需要一个压缩变体",
                pack.pack_kind
            )));
        }
        let mut compression_levels = BTreeSet::new();
        for variant in &pack.variants {
            if variant.compression_level.trim().is_empty() {
                return Err(AppError::InvalidArgument(format!(
                    "LLM 上下文包 {} 存在空 compression_level",
                    pack.pack_kind
                )));
            }
            if !compression_levels.insert(variant.compression_level.as_str()) {
                return Err(AppError::InvalidArgument(format!(
                    "LLM 上下文包 {} 存在重复 compression_level：{}",
                    pack.pack_kind, variant.compression_level
                )));
            }
        }
    }
    Ok(())
}

pub fn render_context_manifest_user_content(
    manifest: &LlmContextManifest,
    selected_packs: &[PlannedContextPack],
) -> Result<String, AppError> {
    let mut sections = Vec::new();
    let stable_packs = selected_packs
        .iter()
        .filter(|pack| pack.stable_prefix)
        .filter_map(|pack| {
            pack.canonical_content_text
                .as_deref()
                .map(|content| (pack, content))
        })
        .collect::<Vec<_>>();
    if !stable_packs.is_empty() {
        let mut context = String::from("[稳定上下文]\n");
        for (pack, content) in stable_packs {
            context.push_str("\n## ");
            context.push_str(pack.pack_ref.pack_kind.as_str());
            context.push_str(" / ");
            context.push_str(pack.pack_ref.compression_level.as_str());
            context.push('\n');
            context.push_str(content);
            context.push('\n');
        }
        sections.push(context);
    }
    let variable_packs = selected_packs
        .iter()
        .filter(|pack| !pack.stable_prefix)
        .filter_map(|pack| {
            pack.canonical_content_text
                .as_deref()
                .map(|content| (pack, content))
        })
        .collect::<Vec<_>>();
    if !variable_packs.is_empty() {
        let mut context = String::from("[可变上下文]\n");
        for (pack, content) in variable_packs {
            context.push_str("\n## ");
            context.push_str(pack.pack_ref.pack_kind.as_str());
            context.push_str(" / ");
            context.push_str(pack.pack_ref.compression_level.as_str());
            context.push('\n');
            context.push_str(content);
            context.push('\n');
        }
        sections.push(context);
    }
    if !manifest.dynamic_user_content.trim().is_empty() {
        sections.push(format!("[当前任务]\n{}", manifest.dynamic_user_content));
    }
    if let Some(output_contract) = manifest.output_contract.as_deref()
        && !output_contract.trim().is_empty()
    {
        sections.push(format!("[输出契约]\n{output_contract}"));
    }
    if sections.is_empty() {
        return Err(AppError::InvalidArgument(
            "LLM 上下文 manifest 没有可发送的用户内容".to_string(),
        ));
    }
    Ok(sections.join("\n\n"))
}

pub fn replace_request_user_content_from_manifest(
    request: &mut ChatRequest,
    manifest: &LlmContextManifest,
    selected_packs: &[PlannedContextPack],
) -> Result<(), AppError> {
    let non_system_count = request
        .messages
        .iter()
        .filter(|message| !matches!(message.role, a4n_engine::llm::provider::MessageRole::System))
        .count();
    let has_non_user_message = request.messages.iter().any(|message| {
        !matches!(
            message.role,
            a4n_engine::llm::provider::MessageRole::System
                | a4n_engine::llm::provider::MessageRole::User
        )
    });
    if non_system_count > 1 || has_non_user_message {
        return Err(AppError::InvalidArgument(
            "LLM 上下文 manifest 只能重写 system + 单 user 结构的请求".to_string(),
        ));
    }
    let user_content = render_context_manifest_user_content(manifest, selected_packs)?;
    let mut messages: Vec<ChatMessage> = request
        .messages
        .iter()
        .filter(|message| matches!(message.role, a4n_engine::llm::provider::MessageRole::System))
        .cloned()
        .collect();
    messages.push(ChatMessage::user(user_content));
    request.messages = messages;
    Ok(())
}

pub fn preview_context_aware_chat_request(
    request: &mut ChatRequest,
    model_context_tokens: usize,
    output_token_reserve: usize,
) -> Result<Option<LlmContextRequestPreview>, AppError> {
    let Some(manifest_json) = request.context_manifest_json.clone() else {
        return Ok(None);
    };
    let manifest: LlmContextManifest = serde_json::from_value(manifest_json.clone())
        .map_err(|err| AppError::InvalidArgument(format!("LLM 上下文 manifest 解析失败: {err}")))?;
    validate_context_manifest(&manifest)?;

    let system_messages = request
        .messages
        .iter()
        .filter(|message| matches!(message.role, MessageRole::System))
        .cloned()
        .collect::<Vec<_>>();
    let mut requirements = Vec::new();
    if !system_messages.is_empty() {
        let system_tokens = system_messages
            .iter()
            .map(|message| estimate_context_tokens(&message.content))
            .sum();
        requirements.push(LlmContextPackRequirement {
            pack_kind: "system_prompt_prefix".to_string(),
            required: true,
            stable_prefix: true,
            variants: vec![LlmContextPackRef {
                pack_kind: "system_prompt_prefix".to_string(),
                content_sha256: source_content_sha256(
                    &serde_json::to_string(&system_messages).map_err(AppError::Serialization)?,
                ),
                compression_level: COMPRESSION_FULL_RAW.to_string(),
                token_estimate: system_tokens,
            }],
        });
    }

    let mut content_by_key = std::collections::HashMap::<(String, String, String), String>::new();
    for pack in &manifest.packs {
        let mut variants = Vec::new();
        for variant in &pack.variants {
            let canonical_content_text = canonical_json_text(&variant.canonical_content_json)?;
            let pack_ref = LlmContextPackRef {
                pack_kind: pack.pack_kind.clone(),
                content_sha256: source_content_sha256(&canonical_content_text),
                compression_level: variant.compression_level.clone(),
                token_estimate: estimate_context_tokens(&canonical_content_text),
            };
            content_by_key.insert(
                (
                    pack_ref.pack_kind.clone(),
                    pack_ref.compression_level.clone(),
                    pack_ref.content_sha256.clone(),
                ),
                canonical_content_text,
            );
            variants.push(pack_ref);
        }
        requirements.push(LlmContextPackRequirement {
            pack_kind: pack.pack_kind.clone(),
            required: pack.required,
            stable_prefix: pack.stable_prefix,
            variants,
        });
    }

    let dynamic_delta_tokens =
        estimate_context_tokens(&render_context_manifest_user_content(&manifest, &[])?);
    let mut plan = ContextBudgetPlanner::default().plan_requirements(
        model_context_tokens,
        output_token_reserve,
        requirements,
        dynamic_delta_tokens,
    )?;
    for selected in &mut plan.selected_packs {
        selected.canonical_content_text = content_by_key
            .get(&(
                selected.pack_ref.pack_kind.clone(),
                selected.pack_ref.compression_level.clone(),
                selected.pack_ref.content_sha256.clone(),
            ))
            .cloned();
    }
    replace_request_user_content_from_manifest(request, &manifest, &plan.selected_packs)?;

    Ok(Some(LlmContextRequestPreview {
        manifest_json,
        plan_json: serde_json::to_value(&plan).map_err(AppError::Serialization)?,
        pack_refs_json: serde_json::to_value(&plan.selected_pack_refs)
            .map_err(AppError::Serialization)?,
        stable_prefix_tokens: plan.stable_prefix_tokens,
        dynamic_delta_tokens: plan.dynamic_delta_tokens,
        available_input_tokens: plan.available_input_tokens,
    }))
}

pub fn canonical_json_text(value: &Value) -> Result<String, AppError> {
    serde_json::to_string_pretty(value).map_err(AppError::Serialization)
}

pub fn estimate_context_tokens(text: &str) -> usize {
    let chinese_chars = text.chars().filter(|c| u32::from(*c) > 0x2E80).count();
    let other_words = text.split_whitespace().count();
    ((chinese_chars as f32 * 0.5) + (other_words as f32 * 1.3)).ceil() as usize
}

fn compression_rank(level: &str) -> usize {
    match level {
        COMPRESSION_FULL_RAW => 4,
        COMPRESSION_FOCUSED_RAW => 3,
        COMPRESSION_COMPRESSED_SCENE => 2,
        COMPRESSION_MINIMAL => 1,
        _ => 0,
    }
}

fn usize_to_i32(value: usize, field: &str) -> Result<i32, AppError> {
    i32::try_from(value)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("LLM 上下文字段 {field} 超出 i32 范围")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pack_ref(kind: &str, level: &str, tokens: usize) -> LlmContextPackRef {
        LlmContextPackRef {
            pack_kind: kind.to_string(),
            content_sha256: format!("{kind}-hash"),
            compression_level: level.to_string(),
            token_estimate: tokens,
        }
    }

    fn requirement(
        kind: &str,
        level: &str,
        tokens: usize,
        required: bool,
    ) -> LlmContextPackRequirement {
        LlmContextPackRequirement {
            pack_kind: kind.to_string(),
            required,
            stable_prefix: true,
            variants: vec![pack_ref(kind, level, tokens)],
        }
    }

    fn manifest_pack(kind: &str) -> LlmContextManifestPack {
        LlmContextManifestPack {
            scope_kind: "chapter".to_string(),
            pack_kind: kind.to_string(),
            source_refs_json: None,
            config_hash: format!("{kind}:config"),
            required: true,
            stable_prefix: true,
            variants: vec![LlmContextManifestPackVariant {
                compression_level: COMPRESSION_FULL_RAW.to_string(),
                canonical_content_json: json!({"kind": kind, "content": "上下文内容"}),
            }],
        }
    }

    fn manifest_with_pack(dynamic_user_content: &str) -> LlmContextManifest {
        LlmContextManifest {
            schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION,
            packs: vec![manifest_pack("chapter_source")],
            dynamic_user_content: dynamic_user_content.to_string(),
            output_contract: None,
        }
    }

    #[test]
    fn context_pack_draft_uses_stable_canonical_text_and_hash() {
        let first = LlmContextPackDraft::from_json(LlmContextPackDraftInput {
            project_id: 1,
            root_project_id: 1,
            scope_kind: "chapter".to_string(),
            workflow: "video".to_string(),
            pack_kind: "source".to_string(),
            source_refs_json: None,
            config_hash: "cfg".to_string(),
            compression_level: COMPRESSION_FULL_RAW.to_string(),
            canonical_content_json: json!({"b": 2, "a": 1}),
        })
        .expect("first pack");
        let second = LlmContextPackDraft::from_json(LlmContextPackDraftInput {
            project_id: 1,
            root_project_id: 1,
            scope_kind: "chapter".to_string(),
            workflow: "video".to_string(),
            pack_kind: "source".to_string(),
            source_refs_json: None,
            config_hash: "cfg".to_string(),
            compression_level: COMPRESSION_FULL_RAW.to_string(),
            canonical_content_json: json!({"a": 1, "b": 2}),
        })
        .expect("second pack");

        assert_eq!(first.canonical_content_text, second.canonical_content_text);
        assert_eq!(first.content_sha256, second.content_sha256);
    }

    #[test]
    fn budget_planner_drops_optional_packs_after_required_content() {
        let plan = ContextBudgetPlanner::new(100)
            .with_input_utilization_percent(100)
            .plan_requirements(
                1_000,
                200,
                vec![
                    requirement("source", COMPRESSION_FOCUSED_RAW, 300, true),
                    requirement("assets", COMPRESSION_FOCUSED_RAW, 500, false),
                ],
                150,
            )
            .expect("plan");

        assert_eq!(plan.available_input_tokens, 700);
        assert_eq!(plan.stable_prefix_tokens, 300);
        assert_eq!(plan.dynamic_delta_tokens, 150);
        assert_eq!(plan.selected_pack_refs.len(), 1);
        assert_eq!(plan.dropped_optional_pack_refs.len(), 1);
    }

    #[test]
    fn budget_planner_rejects_required_pack_that_cannot_fit() {
        let err = ContextBudgetPlanner::new(100)
            .plan_requirements(
                1_000,
                200,
                vec![requirement("source", COMPRESSION_FULL_RAW, 800, true)],
                150,
            )
            .expect_err("required pack should fail");

        assert!(err.to_string().contains("必需上下文包"));
    }

    #[test]
    fn budget_planner_uses_input_utilization_cap() {
        let plan = ContextBudgetPlanner::new(100)
            .plan_requirements(1_000, 200, vec![], 150)
            .expect("plan");

        assert_eq!(plan.raw_available_input_tokens, 700);
        assert_eq!(plan.available_input_tokens, 595);
        assert_eq!(plan.input_utilization_percent, 85);
    }

    #[test]
    fn budget_planner_selects_compressed_required_variant() {
        let plan = ContextBudgetPlanner::new(100)
            .with_input_utilization_percent(100)
            .plan_requirements(
                1_000,
                200,
                vec![LlmContextPackRequirement {
                    pack_kind: "chapter".to_string(),
                    required: true,
                    stable_prefix: true,
                    variants: vec![
                        pack_ref("chapter", COMPRESSION_FULL_RAW, 650),
                        pack_ref("chapter", COMPRESSION_COMPRESSED_SCENE, 300),
                    ],
                }],
                150,
            )
            .expect("compressed variant should fit");

        assert_eq!(plan.selected_pack_refs.len(), 1);
        assert_eq!(
            plan.selected_pack_refs[0].compression_level,
            COMPRESSION_COMPRESSED_SCENE
        );
        assert_eq!(plan.estimated_input_tokens, 450);
    }

    #[test]
    fn budget_planner_reserves_space_for_later_required_packs() {
        let plan = ContextBudgetPlanner::new(100)
            .with_input_utilization_percent(100)
            .plan_requirements(
                1_000,
                200,
                vec![
                    LlmContextPackRequirement {
                        pack_kind: "large_source".to_string(),
                        required: true,
                        stable_prefix: true,
                        variants: vec![
                            pack_ref("large_source", COMPRESSION_FULL_RAW, 500),
                            pack_ref("large_source", COMPRESSION_MINIMAL, 100),
                        ],
                    },
                    requirement("contract", COMPRESSION_FULL_RAW, 300, true),
                ],
                150,
            )
            .expect("planner should downgrade earlier pack for later required pack");

        assert_eq!(plan.selected_pack_refs.len(), 2);
        assert_eq!(
            plan.selected_pack_refs[0].compression_level,
            COMPRESSION_MINIMAL
        );
        assert_eq!(plan.estimated_input_tokens, 550);
    }

    #[test]
    fn budget_planner_rejects_required_minimal_overflow() {
        let err = ContextBudgetPlanner::new(100)
            .with_input_utilization_percent(100)
            .plan_requirements(
                1_000,
                200,
                vec![LlmContextPackRequirement {
                    pack_kind: "chapter".to_string(),
                    required: true,
                    stable_prefix: true,
                    variants: vec![
                        pack_ref("chapter", COMPRESSION_FULL_RAW, 900),
                        pack_ref("chapter", COMPRESSION_MINIMAL, 800),
                    ],
                }],
                150,
            )
            .expect_err("required minimal variant should overflow");

        assert!(err.to_string().contains("最小压缩变体"));
    }

    #[test]
    fn context_aware_request_attaches_manifest_without_serializing_it() {
        let mut manifest = manifest_with_pack("生成当前片段");
        manifest.output_contract = Some("只输出 JSON".to_string());
        let request = context_aware_system_user_request(
            "system",
            manifest,
            Some(0.2),
            None,
            Some("media_prompt".to_string()),
        )
        .expect("request");

        assert!(request.context_manifest_json.is_some());
        assert_eq!(request.messages.len(), 2);
        assert!(request.messages[1].content.contains("[当前任务]"));
        assert!(request.messages[1].content.contains("[输出契约]"));
        let value = serde_json::to_value(&request).expect("serialize request");
        assert!(value.get("context_manifest_json").is_none());
    }

    #[test]
    fn context_aware_request_rejects_empty_manifest_packs() {
        let manifest = LlmContextManifest {
            schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION,
            packs: Vec::new(),
            dynamic_user_content: "生成当前片段".to_string(),
            output_contract: None,
        };

        let err = context_aware_system_user_request(
            "system",
            manifest,
            Some(0.2),
            None,
            Some("media_prompt".to_string()),
        )
        .expect_err("empty context manifest must fail");

        assert!(err.to_string().contains("至少需要一个上下文包"));
    }

    #[test]
    fn manifest_render_skips_empty_stable_context_section() {
        let manifest = manifest_with_pack("当前任务");
        let selected_system_pack = PlannedContextPack {
            pack_ref: pack_ref("system_prompt_prefix", COMPRESSION_FULL_RAW, 10),
            stable_prefix: true,
            canonical_content_text: None,
        };

        let rendered = render_context_manifest_user_content(&manifest, &[selected_system_pack])
            .expect("rendered content");

        assert!(!rendered.contains("[稳定上下文]"));
        assert!(rendered.contains("[当前任务]"));
    }

    #[test]
    fn manifest_render_separates_variable_context_from_stable_prefix() {
        let manifest = manifest_with_pack("当前任务");
        let stable_pack = PlannedContextPack {
            pack_ref: pack_ref("stable", COMPRESSION_FULL_RAW, 10),
            stable_prefix: true,
            canonical_content_text: Some("稳定内容".to_string()),
        };
        let variable_pack = PlannedContextPack {
            pack_ref: pack_ref("asset_binding", COMPRESSION_MINIMAL, 10),
            stable_prefix: false,
            canonical_content_text: Some("可变素材".to_string()),
        };

        let rendered =
            render_context_manifest_user_content(&manifest, &[stable_pack, variable_pack])
                .expect("rendered content");

        let stable_index = rendered.find("[稳定上下文]").expect("stable section");
        let variable_index = rendered.find("[可变上下文]").expect("variable section");
        let task_index = rendered.find("[当前任务]").expect("task section");
        assert!(stable_index < variable_index);
        assert!(variable_index < task_index);
        assert!(rendered.contains("稳定内容"));
        assert!(rendered.contains("可变素材"));
    }

    #[test]
    fn manifest_rewrite_rejects_complex_chat_history() {
        let manifest = manifest_with_pack("当前任务");
        let mut request = ChatRequest::new(
            vec![
                ChatMessage::system("system"),
                ChatMessage::user("first"),
                ChatMessage::assistant("assistant"),
            ],
            Some(0.2),
            None,
            None,
        );

        let err = replace_request_user_content_from_manifest(&mut request, &manifest, &[])
            .expect_err("complex history must fail");

        assert!(err.to_string().contains("只能重写"));
    }

    #[test]
    fn ledger_preserves_pack_order_and_hashes_config_identity() {
        let mut ledger = LlmContextLedger::new();
        ledger
            .add_pack(LlmContextLedgerPackInput {
                scope_kind: ("chapter").into(),
                pack_kind: ("first").into(),
                source_refs_json: None,
                config_identity_json: json!({"version": 1}),
                required: true,
                stable_prefix: true,
                variants: vec![LlmContextLedgerPackDraft::new(
                    COMPRESSION_FULL_RAW,
                    json!({"body": "first"}),
                )],
            })
            .expect("first pack");
        ledger
            .add_pack(LlmContextLedgerPackInput {
                scope_kind: ("chapter").into(),
                pack_kind: ("second").into(),
                source_refs_json: None,
                config_identity_json: json!({"version": 1}),
                required: false,
                stable_prefix: true,
                variants: vec![LlmContextLedgerPackDraft::new(
                    COMPRESSION_MINIMAL,
                    json!({"body": "second"}),
                )],
            })
            .expect("second pack");

        let manifest = ledger.into_manifest("dynamic", None);

        assert_eq!(manifest.packs[0].pack_kind, "first");
        assert_eq!(manifest.packs[1].pack_kind, "second");
        assert_ne!(manifest.packs[0].config_hash, manifest.packs[1].config_hash);
    }

    #[test]
    fn ledger_rejects_duplicate_compression_levels() {
        let mut ledger = LlmContextLedger::new();
        let err = ledger
            .add_pack(LlmContextLedgerPackInput {
                scope_kind: ("chapter").into(),
                pack_kind: ("source").into(),
                source_refs_json: None,
                config_identity_json: json!({"version": 1}),
                required: true,
                stable_prefix: true,
                variants: vec![
                    LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, json!({"a": 1})),
                    LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, json!({"a": 2})),
                ],
            })
            .expect_err("duplicate level should fail");

        assert!(err.to_string().contains("重复 compression_level"));
    }

    #[test]
    fn context_plan_serialization_skips_hydrated_pack_content() {
        let plan = LlmContextPlan {
            model_context_tokens: 1_000,
            output_token_reserve: 200,
            safety_margin_tokens: 100,
            input_utilization_percent: 100,
            raw_available_input_tokens: 700,
            available_input_tokens: 700,
            stable_prefix_tokens: 10,
            dynamic_delta_tokens: 20,
            selected_compression_level: COMPRESSION_FULL_RAW.to_string(),
            selected_pack_refs: vec![pack_ref("chapter", COMPRESSION_FULL_RAW, 10)],
            selected_packs: vec![PlannedContextPack {
                pack_ref: pack_ref("chapter", COMPRESSION_FULL_RAW, 10),
                stable_prefix: true,
                canonical_content_text: Some("full content should not be serialized".to_string()),
            }],
            dropped_optional_pack_refs: Vec::new(),
            estimated_input_tokens: 30,
        };

        let serialized = serde_json::to_value(&plan).expect("serialize plan");

        assert!(serialized.get("selected_packs").is_none());
        assert!(
            !serialized
                .to_string()
                .contains("full content should not be serialized")
        );
    }
}
