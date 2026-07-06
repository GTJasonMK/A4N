//! 生成意图与提示词产物服务。

use std::{collections::BTreeSet, sync::Arc};

use chrono::Utc;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use a4n_core::error::AppError;
use a4n_core::models::{
    asset::object_profile,
    character::protagonist_profile,
    generation::{
        generation_run, generation_run_target, generation_spec, generation_spec_entity_ref,
        generation_spec_media_ref, generation_spec_text_anchor, llm_call_trace,
    },
    media::media_asset,
    project::{novel_project, source_snapshot_profile},
    scene::scene_profile,
};
use a4n_engine::llm::provider::ChatRequest;
use a4n_engine::llm::provider::UsageInfo;

use crate::llm_context::{LlmContextManifest, estimate_context_tokens};
use crate::source::source_content_sha256;

pub const SPEC_STATUS_DRAFT: &str = "draft";
const SPEC_STATUS_READY: &str = "ready";
const SPEC_STATUS_RUNNING: &str = "running";
const SPEC_STATUS_COMPLETED: &str = "completed";
const SPEC_STATUS_FAILED: &str = "failed";
const SPEC_STATUS_CANCELLED: &str = "cancelled";
const RUN_STATUS_RUNNING: &str = "running";
const RUN_STATUS_COMPLETED: &str = "completed";
const RUN_STATUS_FAILED: &str = "failed";

pub const GENERATION_SPEC_TARGET_KINDS: &[&str] = &[
    "character_material_image",
    "portrait_expression",
    "object_material_image",
    "scene_material_image",
    "galgame_background",
    "galgame_cg",
    "video_segment",
    "video_bridge_segment",
    "video_shot_analysis",
    "video_segment_plan",
    "video_frame_prompt",
    "video_intermediate",
    "boundary_frame",
    "first_frame",
    "final_assembly",
    "manga_extraction_summary",
    "manga_page_plan",
    "manga_page_prompt_batch",
    "manga_page",
    "manga_intermediate",
    "canvas_prompt",
    "canvas_image",
    "canvas_video",
    "canvas_assembly",
];
pub const GENERATION_SPEC_TARGET_ENTITY_KINDS: &[&str] = &[
    "character",
    "character_profile",
    "object",
    "scene",
    "chapter",
    "segment",
    "assembly",
    "chapter_manga_prompt",
    "chapter_video_prompt",
    "galgame_scene",
    "galgame_line",
    "canvas_node",
];
const GENERATION_ENTITY_KINDS: &[&str] =
    &["character", "character_profile", "object", "scene", "event"];
pub const GENERATION_SPEC_STATUSES: &[&str] = &[
    SPEC_STATUS_DRAFT,
    SPEC_STATUS_READY,
    SPEC_STATUS_RUNNING,
    SPEC_STATUS_COMPLETED,
    SPEC_STATUS_FAILED,
    SPEC_STATUS_CANCELLED,
];
pub const GENERATION_TEXT_ANCHOR_ROLES: &[&str] = &[
    "context",
    "direct_content",
    "previous_context",
    "next_context",
];
pub const GENERATION_ENTITY_ROLES: &[&str] = &[
    "subject",
    "supporting",
    "location",
    "prop",
    "continuity_reference",
];
pub const GENERATION_MEDIA_REF_ROLES: &[&str] = &[
    "first_frame",
    "end_frame",
    "character_reference",
    "object_reference",
    "scene_reference",
    "style_reference",
    "reference",
];
pub const GENERATION_PROMPT_KINDS: &[&str] = &[
    "image_prompt",
    "video_prompt",
    "chat_video_prompt",
    "provider_video_prompt",
    "negative_prompt",
    "manga_pipeline_output",
    "manga_page_prompt",
    "video_shot_analysis",
    "video_segment_plan",
    "canvas_prompt",
    "background_image_prompt",
    "event_cg_prompt",
];

pub fn is_valid_generation_spec_target_kind(value: &str) -> bool {
    GENERATION_SPEC_TARGET_KINDS.contains(&value)
}

pub fn is_valid_generation_spec_target_entity_kind(value: &str) -> bool {
    GENERATION_SPEC_TARGET_ENTITY_KINDS.contains(&value)
}

pub fn is_valid_generation_entity_kind(value: &str) -> bool {
    GENERATION_ENTITY_KINDS.contains(&value)
}

pub fn is_valid_generation_spec_status(value: &str) -> bool {
    GENERATION_SPEC_STATUSES.contains(&value)
}

pub fn is_valid_generation_text_anchor_role(value: &str) -> bool {
    GENERATION_TEXT_ANCHOR_ROLES.contains(&value)
}

pub fn is_valid_generation_entity_role(value: &str) -> bool {
    GENERATION_ENTITY_ROLES.contains(&value)
}

pub fn is_valid_generation_media_ref_role(value: &str) -> bool {
    GENERATION_MEDIA_REF_ROLES.contains(&value)
}

pub fn is_valid_generation_prompt_kind(value: &str) -> bool {
    GENERATION_PROMPT_KINDS.contains(&value)
}

const INTERNAL_PROMPT_TOKENS: [&str; 5] = [
    "workflow_mode",
    "contract_version",
    "scene_ref",
    "must_repeat_from_previous",
    "duration_capability",
];
const VIDEO_PROMPT_REQUIRED_SECTIONS: [&str; 6] = [
    "[主体定义与素材绑定]",
    "[片段内容以及前因后果]",
    "[镜头分镜]",
    "[画面风格描述]",
    "[连续性与结尾状态]",
    "[限制与避免项]",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateGenerationRunInput {
    pub project_id: i64,
    pub root_project_id: i64,
    pub workflow: String,
    pub step_key: String,
    pub title: String,
    pub model_kind: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub checkpoint: GenerationRunCheckpointContext,
    #[serde(default)]
    pub prompt_template_names_json: Option<Value>,
    #[serde(default)]
    pub prompt_template_hashes_json: Option<Value>,
    #[serde(default)]
    pub rendered_messages_json: Option<Value>,
    #[serde(default)]
    pub rendered_prompts_json: Option<Value>,
    #[serde(default)]
    pub input_variables_json: Option<Value>,
    #[serde(default)]
    pub source_refs_json: Option<Value>,
    #[serde(default)]
    pub media_refs_json: Option<Value>,
    #[serde(default)]
    pub request_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AttachGenerationRunTargetInput {
    pub generation_run_id: i64,
    pub target_kind: String,
    pub target_table: String,
    pub target_id: i64,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRunTraceTarget {
    pub target_kind: String,
    pub target_table: String,
    pub target_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordCompletedGenerationRunInput {
    pub project_id: i64,
    pub workflow: String,
    pub step_key: String,
    pub title: String,
    pub model_kind: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub prompt_template_names_json: Option<Value>,
    pub prompt_template_hashes_json: Option<Value>,
    pub rendered_messages_json: Option<Value>,
    pub rendered_prompts_json: Option<Value>,
    pub input_variables_json: Option<Value>,
    pub source_refs_json: Option<Value>,
    pub media_refs_json: Option<Value>,
    pub request_json: Option<Value>,
    pub response_summary_json: Option<Value>,
    pub targets: Vec<GenerationRunTraceTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordStartedGenerationRunInput {
    pub project_id: i64,
    pub workflow: String,
    pub step_key: String,
    pub title: String,
    pub model_kind: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub prompt_template_names_json: Option<Value>,
    pub prompt_template_hashes_json: Option<Value>,
    pub rendered_messages_json: Option<Value>,
    pub rendered_prompts_json: Option<Value>,
    pub input_variables_json: Option<Value>,
    pub source_refs_json: Option<Value>,
    pub media_refs_json: Option<Value>,
    pub request_json: Option<Value>,
    pub targets: Vec<GenerationRunTraceTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordLlmCallTraceInput {
    pub generation_run_id: Option<i64>,
    pub workflow_run_id: Option<i64>,
    pub call_key: Option<String>,
    pub input_fingerprint_sha256: Option<String>,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub workflow: String,
    pub step_key: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub context_plan_json: Option<Value>,
    pub context_pack_refs_json: Option<Value>,
    pub usage: Option<UsageInfo>,
    pub duration_ms: Option<i64>,
    pub streamed: bool,
    pub status: String,
    pub error_kind: Option<String>,
    pub error_message: Option<String>,
    pub finish_reason: Option<String>,
}

pub fn llm_usage_summary_text(usage: Option<&UsageInfo>) -> String {
    let Some(usage) = usage else {
        return "模型未返回 token usage".to_string();
    };
    let mut summary = format!(
        "Token {}（输入 {} / 输出 {}）",
        usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
    );
    match (
        usage.prompt_cache_hit_tokens,
        usage.prompt_cache_miss_tokens,
    ) {
        (Some(hit), Some(miss)) => {
            let total = hit + miss;
            if total > 0 {
                let ratio = (hit as f64 / total as f64) * 100.0;
                summary.push_str(&format!(
                    "，缓存命中 {:.1}%（命中 {} / 未命中 {}）",
                    ratio, hit, miss
                ));
            } else {
                summary.push_str("，缓存命中 0.0%（命中 0 / 未命中 0）");
            }
        }
        (Some(hit), None) => {
            summary.push_str(&format!("，缓存命中 token {}", hit));
        }
        (None, Some(miss)) => {
            summary.push_str(&format!("，缓存未命中 token {}", miss));
        }
        (None, None) => {
            summary.push_str("，缓存数据未返回");
        }
    }
    summary
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerationRunCheckpointContext {
    pub workflow_run_id: Option<i64>,
    pub call_key: Option<String>,
    pub input_fingerprint_sha256: Option<String>,
    pub input_fingerprint_json: Option<Value>,
    pub raw_response_json: Option<Value>,
    pub parsed_output_json: Option<Value>,
    pub validation_json: Option<Value>,
    pub artifact_refs_json: Option<Value>,
    pub attempt: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordCompletedGenerationRunWithCheckpointInput {
    pub run: RecordCompletedGenerationRunInput,
    pub checkpoint: GenerationRunCheckpointContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachGenerationRunsToWorkflowInput {
    pub target_table: String,
    pub target_id: i64,
    pub workflow_run_id: i64,
    pub call_key: Option<String>,
    pub input_fingerprint_sha256: Option<String>,
    pub input_fingerprint_json: Option<Value>,
    pub artifact_refs_json: Option<Value>,
}

pub fn chat_request_messages_json(request: &ChatRequest) -> Result<Value, AppError> {
    serde_json::to_value(&request.messages).map_err(AppError::Serialization)
}

pub fn chat_request_trace_json(request: &ChatRequest) -> Result<Value, AppError> {
    serde_json::to_value(request).map_err(AppError::Serialization)
}

pub fn context_aware_chat_request_trace_json(request: &ChatRequest) -> Result<Value, AppError> {
    let mut request_json = chat_request_trace_json(request)?;
    let Some(object) = request_json.as_object_mut() else {
        return Ok(request_json);
    };
    if let Some(manifest_json) = request.context_manifest_json.as_ref() {
        object.insert(
            "context_manifest_summary".to_string(),
            context_manifest_trace_summary(manifest_json)?,
        );
    }
    if let Some(trace_context) = request.trace_context.as_ref() {
        object.insert(
            "trace_context".to_string(),
            json!({
                "project_id": trace_context.project_id,
                "root_project_id": trace_context.root_project_id,
                "workflow": trace_context.workflow,
                "step_key": trace_context.step_key,
                "generation_run_id": trace_context.generation_run_id,
                "workflow_run_id": trace_context.workflow_run_id,
                "call_key": trace_context.call_key,
                "input_fingerprint_sha256": trace_context.input_fingerprint_sha256,
                "context_plan": trace_context.context_plan_json,
                "context_pack_refs": trace_context.context_pack_refs_json,
            }),
        );
    }
    Ok(request_json)
}

pub fn context_manifest_trace_summary(manifest_json: &Value) -> Result<Value, AppError> {
    let manifest: LlmContextManifest =
        serde_json::from_value(manifest_json.clone()).map_err(AppError::Serialization)?;
    let packs = manifest
        .packs
        .iter()
        .map(|pack| {
            let variants = pack
                .variants
                .iter()
                .map(|variant| {
                    let text = serde_json::to_string(&variant.canonical_content_json)
                        .map_err(AppError::Serialization)?;
                    Ok(json!({
                        "compression_level": variant.compression_level,
                        "content_sha256": source_content_sha256(&text),
                        "chars": text.chars().count(),
                        "tokens_estimate": estimate_context_tokens(&text),
                    }))
                })
                .collect::<Result<Vec<_>, AppError>>()?;
            Ok(json!({
                "scope_kind": pack.scope_kind,
                "pack_kind": pack.pack_kind,
                "source_refs": pack.source_refs_json,
                "config_hash": pack.config_hash,
                "required": pack.required,
                "stable_prefix": pack.stable_prefix,
                "variants": variants
            }))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(json!({
        "schema_version": manifest.schema_version,
        "dynamic_user_chars": manifest.dynamic_user_content.chars().count(),
        "dynamic_user_tokens_estimate": estimate_context_tokens(&manifest.dynamic_user_content),
        "output_contract_chars": manifest.output_contract.as_deref().map(str::chars).map(Iterator::count),
        "output_contract_tokens_estimate": manifest.output_contract.as_deref().map(estimate_context_tokens),
        "packs": packs
    }))
}

pub fn generation_checkpoint_fingerprint_sha256(value: &Value) -> Result<String, AppError> {
    let serialized = serde_json::to_string(value).map_err(AppError::Serialization)?;
    Ok(source_content_sha256(&serialized))
}

pub fn generation_artifact_target_json(target: &GenerationRunTraceTarget) -> Value {
    json!({
        "target_kind": &target.target_kind,
        "target_table": &target.target_table,
        "target_id": target.target_id,
    })
}

pub fn append_generation_artifact_target(
    artifact_refs: Option<Value>,
    target: GenerationRunTraceTarget,
) -> Result<Value, AppError> {
    let target = generation_artifact_target_json(&target);
    match artifact_refs {
        Some(Value::Object(mut map)) => {
            let targets = match map.remove("targets") {
                Some(Value::Array(mut targets)) => {
                    targets.push(target);
                    Value::Array(targets)
                }
                Some(_) => {
                    return Err(AppError::InvalidArgument(
                        "generation artifact_refs_json.targets 必须是数组".to_string(),
                    ));
                }
                None => Value::Array(vec![target]),
            };
            map.insert("targets".to_string(), targets);
            Ok(Value::Object(map))
        }
        Some(_) => Err(AppError::InvalidArgument(
            "generation artifact_refs_json 必须是对象".to_string(),
        )),
        None => Ok(json!({
            "targets": [target],
        })),
    }
}

pub fn text_response_summary_json(
    content: &str,
    finish_reason: Option<&str>,
    extra: Value,
) -> Value {
    let mut summary = json!({
        "content_chars": content.chars().count(),
        "content_sha256": source_content_sha256(content),
        "content_preview": content.chars().take(240).collect::<String>(),
        "finish_reason": finish_reason,
    });
    if let (Some(summary), Some(extra)) = (summary.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            summary.insert(key.clone(), value.clone());
        }
    }
    summary
}

pub async fn record_llm_call_trace<C>(
    db: &C,
    input: RecordLlmCallTraceInput,
) -> Result<llm_call_trace::Model, AppError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let usage_trace = usage_trace_fields(input.usage.as_ref())?;

    llm_call_trace::ActiveModel {
        id: NotSet,
        generation_run_id: Set(input.generation_run_id),
        workflow_run_id: Set(input.workflow_run_id),
        call_key: Set(input.call_key),
        input_fingerprint_sha256: Set(input.input_fingerprint_sha256),
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        workflow: Set(input.workflow),
        step_key: Set(input.step_key),
        provider: Set(input.provider),
        model: Set(input.model),
        context_plan_json: Set(input.context_plan_json),
        context_pack_refs_json: Set(input.context_pack_refs_json),
        prompt_tokens: Set(usage_trace.prompt_tokens),
        completion_tokens: Set(usage_trace.completion_tokens),
        total_tokens: Set(usage_trace.total_tokens),
        prompt_cache_hit_tokens: Set(usage_trace.prompt_cache_hit_tokens),
        prompt_cache_miss_tokens: Set(usage_trace.prompt_cache_miss_tokens),
        prompt_cache_hit_ratio: Set(usage_trace.prompt_cache_hit_ratio),
        duration_ms: Set(input.duration_ms),
        streamed: Set(input.streamed),
        status: Set(input.status),
        error_kind: Set(input.error_kind),
        error_message: Set(input.error_message),
        finish_reason: Set(input.finish_reason),
        created_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

#[derive(Default)]
struct UsageTraceFields {
    prompt_tokens: Option<i32>,
    completion_tokens: Option<i32>,
    total_tokens: Option<i32>,
    prompt_cache_hit_tokens: Option<i32>,
    prompt_cache_miss_tokens: Option<i32>,
    prompt_cache_hit_ratio: Option<f64>,
}

fn usage_trace_fields(usage: Option<&UsageInfo>) -> Result<UsageTraceFields, AppError> {
    let Some(usage) = usage else {
        return Ok(UsageTraceFields::default());
    };
    let prompt_cache_hit_ratio = match (
        usage.prompt_cache_hit_tokens,
        usage.prompt_cache_miss_tokens,
    ) {
        (Some(hit), Some(miss)) if hit + miss > 0 => Some(hit as f64 / (hit + miss) as f64),
        _ => None,
    };
    Ok(UsageTraceFields {
        prompt_tokens: Some(usize_to_i32(usage.prompt_tokens, "prompt_tokens")?),
        completion_tokens: Some(usize_to_i32(usage.completion_tokens, "completion_tokens")?),
        total_tokens: Some(usize_to_i32(usage.total_tokens, "total_tokens")?),
        prompt_cache_hit_tokens: usage
            .prompt_cache_hit_tokens
            .map(|value| usize_to_i32(value, "prompt_cache_hit_tokens"))
            .transpose()?,
        prompt_cache_miss_tokens: usage
            .prompt_cache_miss_tokens
            .map(|value| usize_to_i32(value, "prompt_cache_miss_tokens"))
            .transpose()?,
        prompt_cache_hit_ratio,
    })
}

fn usize_to_i32(value: usize, field: &str) -> Result<i32, AppError> {
    i32::try_from(value)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("LLM usage 字段 {field} 超出 i32 范围")))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextAnchorRefInput {
    pub text_anchor_id: i64,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityRefInput {
    pub entity_kind: String,
    pub entity_id: i64,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaRefInput {
    pub media_asset_id: i64,
    pub role: String,
}

#[derive(Clone)]
pub struct GenerationRunService {
    db: Arc<DatabaseConnection>,
}

impl GenerationRunService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn record_completed_run_with_checkpoint(
        &self,
        input: RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<generation_run::Model, AppError> {
        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let run = record_completed_generation_run_with_checkpoint(
            &tx,
            input.run,
            normalize_checkpoint_context(input.checkpoint)?,
        )
        .await?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(run)
    }

    pub async fn runs_for_workflow(
        &self,
        workflow_run_id: i64,
        limit: u64,
    ) -> Result<Vec<generation_run::Model>, AppError> {
        validate_positive_id(workflow_run_id, "workflow_run_id")?;
        generation_run::Entity::find()
            .filter(generation_run::Column::WorkflowRunId.eq(workflow_run_id))
            .order_by_asc(generation_run::Column::CreatedAt)
            .limit(limit.clamp(1, 500))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn attach_runs_to_workflow_for_target(
        &self,
        input: AttachGenerationRunsToWorkflowInput,
    ) -> Result<Vec<generation_run::Model>, AppError> {
        let target_table = required_owned_text(input.target_table, "target_table")?;
        validate_positive_id(input.target_id, "target_id")?;
        validate_positive_id(input.workflow_run_id, "workflow_run_id")?;
        let call_key = normalize_optional_owned_text(input.call_key);
        let input_fingerprint_sha256 =
            normalize_optional_owned_text(input.input_fingerprint_sha256);
        let targets = generation_run_target::Entity::find()
            .filter(generation_run_target::Column::TargetTable.eq(target_table))
            .filter(generation_run_target::Column::TargetId.eq(input.target_id))
            .order_by_asc(generation_run_target::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if targets.is_empty() {
            return Ok(Vec::new());
        }
        let run_ids = targets
            .iter()
            .map(|target| target.generation_run_id)
            .collect::<BTreeSet<_>>();
        let runs = generation_run::Entity::find()
            .filter(generation_run::Column::Id.is_in(run_ids))
            .order_by_asc(generation_run::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let mut updated = Vec::with_capacity(runs.len());
        for run in runs {
            let mut active: generation_run::ActiveModel = run.into();
            active.workflow_run_id = Set(Some(input.workflow_run_id));
            if call_key.is_some() {
                active.call_key = Set(call_key.clone());
            }
            if input_fingerprint_sha256.is_some() {
                active.input_fingerprint_sha256 = Set(input_fingerprint_sha256.clone());
            }
            if input.input_fingerprint_json.is_some() {
                active.input_fingerprint_json = Set(input.input_fingerprint_json.clone());
            }
            if input.artifact_refs_json.is_some() {
                active.artifact_refs_json = Set(input.artifact_refs_json.clone());
            }
            updated.push(
                active
                    .update(self.db.as_ref())
                    .await
                    .map_err(AppError::Database)?,
            );
        }
        Ok(updated)
    }

    pub async fn complete_runs_for_target(
        &self,
        target_table: &str,
        target_id: i64,
        response_summary_json: Option<Value>,
    ) -> Result<Vec<generation_run::Model>, AppError> {
        self.update_running_runs_for_target(
            target_table,
            target_id,
            RUN_STATUS_COMPLETED,
            response_summary_json,
            None,
        )
        .await
    }

    pub async fn fail_runs_for_target(
        &self,
        target_table: &str,
        target_id: i64,
        error_message: String,
        response_summary_json: Option<Value>,
    ) -> Result<Vec<generation_run::Model>, AppError> {
        self.update_running_runs_for_target(
            target_table,
            target_id,
            RUN_STATUS_FAILED,
            response_summary_json,
            Some(error_message),
        )
        .await
    }

    async fn update_running_runs_for_target(
        &self,
        target_table: &str,
        target_id: i64,
        status: &str,
        response_summary_json: Option<Value>,
        error_message: Option<String>,
    ) -> Result<Vec<generation_run::Model>, AppError> {
        let target_table = required_owned_text(target_table.to_string(), "target_table")?;
        validate_positive_id(target_id, "target_id")?;
        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let targets = generation_run_target::Entity::find()
            .filter(generation_run_target::Column::TargetTable.eq(target_table))
            .filter(generation_run_target::Column::TargetId.eq(target_id))
            .order_by_asc(generation_run_target::Column::Id)
            .all(&tx)
            .await
            .map_err(AppError::Database)?;
        if targets.is_empty() {
            return Ok(Vec::new());
        }
        let run_ids = targets
            .iter()
            .map(|target| target.generation_run_id)
            .collect::<BTreeSet<_>>();
        let runs = generation_run::Entity::find()
            .filter(generation_run::Column::Id.is_in(run_ids))
            .filter(generation_run::Column::Status.eq(RUN_STATUS_RUNNING))
            .order_by_asc(generation_run::Column::CreatedAt)
            .all(&tx)
            .await
            .map_err(AppError::Database)?;
        let mut updated = Vec::with_capacity(runs.len());
        for run in runs {
            updated.push(
                update_generation_run_status(
                    &tx,
                    run.id,
                    status,
                    response_summary_json.clone(),
                    error_message.clone(),
                )
                .await?,
            );
        }
        tx.commit().await.map_err(AppError::Database)?;
        Ok(updated)
    }
}

pub(crate) async fn record_started_generation_run<C>(
    db: &C,
    input: RecordStartedGenerationRunInput,
) -> Result<generation_run::Model, AppError>
where
    C: ConnectionTrait,
{
    record_started_generation_run_with_checkpoint(
        db,
        input,
        GenerationRunCheckpointContext::default(),
    )
    .await
}

async fn record_started_generation_run_with_checkpoint<C>(
    db: &C,
    input: RecordStartedGenerationRunInput,
    checkpoint: GenerationRunCheckpointContext,
) -> Result<generation_run::Model, AppError>
where
    C: ConnectionTrait,
{
    if input.targets.is_empty() {
        return Err(AppError::InvalidArgument(
            "生成运行记录必须绑定至少一个目标资产".to_string(),
        ));
    }
    let root_project_id = root_project_id_for_project(db, input.project_id).await?;
    let checkpoint = normalize_checkpoint_context(checkpoint)?;
    let run = insert_generation_run(
        db,
        CreateGenerationRunInput {
            project_id: input.project_id,
            root_project_id,
            workflow: input.workflow,
            step_key: input.step_key,
            title: input.title,
            model_kind: input.model_kind,
            provider: input.provider,
            model: input.model,
            checkpoint,
            prompt_template_names_json: input.prompt_template_names_json,
            prompt_template_hashes_json: input.prompt_template_hashes_json,
            rendered_messages_json: input.rendered_messages_json,
            rendered_prompts_json: input.rendered_prompts_json,
            input_variables_json: input.input_variables_json,
            source_refs_json: input.source_refs_json,
            media_refs_json: input.media_refs_json,
            request_json: input.request_json,
        },
    )
    .await?;
    for target in input.targets {
        insert_generation_run_target(
            db,
            AttachGenerationRunTargetInput {
                generation_run_id: run.id,
                target_kind: target.target_kind,
                target_table: target.target_table,
                target_id: target.target_id,
                role: "output".to_string(),
            },
        )
        .await?;
    }
    Ok(run)
}

pub async fn record_completed_generation_run<C>(
    db: &C,
    input: RecordCompletedGenerationRunInput,
) -> Result<generation_run::Model, AppError>
where
    C: ConnectionTrait,
{
    record_completed_generation_run_with_checkpoint(
        db,
        input,
        GenerationRunCheckpointContext::default(),
    )
    .await
}

pub async fn record_completed_generation_run_with_checkpoint<C>(
    db: &C,
    input: RecordCompletedGenerationRunInput,
    checkpoint: GenerationRunCheckpointContext,
) -> Result<generation_run::Model, AppError>
where
    C: ConnectionTrait,
{
    let response_summary_json = input.response_summary_json;
    let run = record_started_generation_run_with_checkpoint(
        db,
        RecordStartedGenerationRunInput {
            project_id: input.project_id,
            workflow: input.workflow,
            step_key: input.step_key,
            title: input.title,
            model_kind: input.model_kind,
            provider: input.provider,
            model: input.model,
            prompt_template_names_json: input.prompt_template_names_json,
            prompt_template_hashes_json: input.prompt_template_hashes_json,
            rendered_messages_json: input.rendered_messages_json,
            rendered_prompts_json: input.rendered_prompts_json,
            input_variables_json: input.input_variables_json,
            source_refs_json: input.source_refs_json,
            media_refs_json: input.media_refs_json,
            request_json: input.request_json,
            targets: input.targets,
        },
        checkpoint,
    )
    .await?;
    update_generation_run_status(
        db,
        run.id,
        RUN_STATUS_COMPLETED,
        response_summary_json,
        None,
    )
    .await
}

pub async fn delete_generation_run_targets_for<C>(
    db: &C,
    target_table: &str,
    target_ids: &[i64],
) -> Result<u64, AppError>
where
    C: ConnectionTrait,
{
    if target_ids.is_empty() {
        return Ok(0);
    }
    let target_table = required_owned_text(target_table.to_string(), "target_table")?;
    for target_id in target_ids {
        validate_positive_id(*target_id, "target_id")?;
    }
    generation_run_target::Entity::delete_many()
        .filter(generation_run_target::Column::TargetTable.eq(target_table))
        .filter(generation_run_target::Column::TargetId.is_in(target_ids.iter().copied()))
        .exec(db)
        .await
        .map(|result| result.rows_affected)
        .map_err(AppError::Database)
}

async fn insert_generation_run<C>(
    db: &C,
    input: CreateGenerationRunInput,
) -> Result<generation_run::Model, AppError>
where
    C: ConnectionTrait,
{
    validate_positive_id(input.project_id, "project_id")?;
    validate_positive_id(input.root_project_id, "root_project_id")?;
    let workflow = required_owned_text(input.workflow, "workflow")?;
    let step_key = required_owned_text(input.step_key, "step_key")?;
    let title = required_owned_text(input.title, "title")?;
    let model_kind = required_owned_text(input.model_kind, "model_kind")?;
    let checkpoint = normalize_checkpoint_context(input.checkpoint)?;
    let now = Utc::now().naive_utc();

    generation_run::ActiveModel {
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        workflow: Set(workflow),
        step_key: Set(step_key),
        title: Set(title),
        model_kind: Set(model_kind),
        provider: Set(normalize_optional_owned_text(input.provider)),
        model: Set(normalize_optional_owned_text(input.model)),
        status: Set(RUN_STATUS_RUNNING.to_string()),
        workflow_run_id: Set(checkpoint.workflow_run_id),
        call_key: Set(checkpoint.call_key),
        input_fingerprint_sha256: Set(checkpoint.input_fingerprint_sha256),
        input_fingerprint_json: Set(checkpoint.input_fingerprint_json),
        raw_response_json: Set(checkpoint.raw_response_json),
        parsed_output_json: Set(checkpoint.parsed_output_json),
        validation_json: Set(checkpoint.validation_json),
        artifact_refs_json: Set(checkpoint.artifact_refs_json),
        attempt: Set(checkpoint.attempt.unwrap_or(1)),
        prompt_template_names_json: Set(input.prompt_template_names_json),
        prompt_template_hashes_json: Set(input.prompt_template_hashes_json),
        rendered_messages_json: Set(input.rendered_messages_json),
        rendered_prompts_json: Set(input.rendered_prompts_json),
        input_variables_json: Set(input.input_variables_json),
        source_refs_json: Set(input.source_refs_json),
        media_refs_json: Set(input.media_refs_json),
        request_json: Set(input.request_json),
        response_summary_json: Set(None),
        error_message: Set(None),
        created_at: Set(now),
        completed_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

async fn insert_generation_run_target<C>(
    db: &C,
    input: AttachGenerationRunTargetInput,
) -> Result<generation_run_target::Model, AppError>
where
    C: ConnectionTrait,
{
    validate_positive_id(input.generation_run_id, "generation_run_id")?;
    validate_positive_id(input.target_id, "target_id")?;
    let target_kind = required_owned_text(input.target_kind, "target_kind")?;
    let target_table = required_owned_text(input.target_table, "target_table")?;
    let role = required_owned_text(input.role, "role")?;
    let run = generation_run::Entity::find_by_id(input.generation_run_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| {
            AppError::NotFound(format!("生成运行记录不存在: {}", input.generation_run_id))
        })?;

    generation_run_target::ActiveModel {
        generation_run_id: Set(run.id),
        target_kind: Set(target_kind),
        target_table: Set(target_table),
        target_id: Set(input.target_id),
        role: Set(role),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

async fn update_generation_run_status<C>(
    db: &C,
    generation_run_id: i64,
    status: &str,
    response_summary_json: Option<Value>,
    error_message: Option<String>,
) -> Result<generation_run::Model, AppError>
where
    C: ConnectionTrait,
{
    validate_positive_id(generation_run_id, "generation_run_id")?;
    let run = generation_run::Entity::find_by_id(generation_run_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("生成运行记录不存在: {generation_run_id}")))?;
    let mut active: generation_run::ActiveModel = run.into();
    active.status = Set(status.to_string());
    active.response_summary_json = Set(response_summary_json);
    active.error_message = Set(error_message);
    active.completed_at = Set(Some(Utc::now().naive_utc()));
    active.update(db).await.map_err(AppError::Database)
}

fn normalize_checkpoint_context(
    input: GenerationRunCheckpointContext,
) -> Result<GenerationRunCheckpointContext, AppError> {
    if let Some(workflow_run_id) = input.workflow_run_id {
        validate_positive_id(workflow_run_id, "workflow_run_id")?;
    }
    if let Some(attempt) = input.attempt
        && attempt <= 0
    {
        return Err(AppError::InvalidArgument("attempt 必须大于 0".to_string()));
    }
    Ok(GenerationRunCheckpointContext {
        workflow_run_id: input.workflow_run_id,
        call_key: normalize_optional_owned_text(input.call_key),
        input_fingerprint_sha256: normalize_optional_owned_text(input.input_fingerprint_sha256),
        input_fingerprint_json: input.input_fingerprint_json,
        raw_response_json: input.raw_response_json,
        parsed_output_json: input.parsed_output_json,
        validation_json: input.validation_json,
        artifact_refs_json: input.artifact_refs_json,
        attempt: input.attempt,
    })
}

async fn root_project_id_for_project<C>(db: &C, project_id: i64) -> Result<i64, AppError>
where
    C: ConnectionTrait,
{
    validate_positive_id(project_id, "project_id")?;
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    Ok(project.root_project_id.unwrap_or(project.id))
}

pub(crate) async fn insert_generation_text_anchor_ref<C>(
    db: &C,
    generation_spec_id: i64,
    text_anchor_id: i64,
    role: &str,
) -> Result<generation_spec_text_anchor::Model, AppError>
where
    C: ConnectionTrait,
{
    generation_spec_text_anchor::ActiveModel {
        generation_spec_id: Set(generation_spec_id),
        text_anchor_id: Set(text_anchor_id),
        role: Set(role.to_string()),
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

pub(crate) async fn insert_generation_entity_ref<C>(
    db: &C,
    generation_spec_id: i64,
    entity_kind: &str,
    entity_id: i64,
    role: &str,
) -> Result<generation_spec_entity_ref::Model, AppError>
where
    C: ConnectionTrait,
{
    generation_spec_entity_ref::ActiveModel {
        generation_spec_id: Set(generation_spec_id),
        entity_kind: Set(entity_kind.to_string()),
        entity_id: Set(entity_id),
        role: Set(role.to_string()),
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

pub(crate) async fn insert_generation_media_ref<C>(
    db: &C,
    spec: &generation_spec::Model,
    media_asset_id: i64,
    role: &str,
) -> Result<generation_spec_media_ref::Model, AppError>
where
    C: ConnectionTrait,
{
    let role = normalize_media_ref_role(role)?;
    let asset = media_asset::Entity::find_by_id(media_asset_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("媒体资产不存在: {media_asset_id}")))?;
    if asset.project_id != spec.project_id || asset.root_project_id != spec.root_project_id {
        return Err(AppError::InvalidArgument(format!(
            "生成意图 {} 不能引用跨项目媒体资产 {}",
            spec.id, asset.id
        )));
    }
    if asset.status != "completed" || asset.file_path.is_none() {
        return Err(AppError::InvalidArgument(format!(
            "媒体引用 {} 不是可用的已完成资产",
            asset.id
        )));
    }
    generation_spec_media_ref::ActiveModel {
        generation_spec_id: Set(spec.id),
        media_asset_id: Set(asset.id),
        role: Set(role.to_string()),
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

pub(crate) async fn validate_required_prompt_refs<C>(
    db: &C,
    spec: &generation_spec::Model,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let anchor_count = generation_spec_text_anchor::Entity::find()
        .filter(generation_spec_text_anchor::Column::GenerationSpecId.eq(spec.id))
        .count(db)
        .await
        .map_err(AppError::Database)?;
    if anchor_count == 0 {
        return Err(AppError::InvalidArgument(format!(
            "生成意图 {} 缺少必需文本锚点",
            spec.id
        )));
    }

    if matches!(
        spec.target_kind.as_str(),
        "character_material_image" | "object_material_image" | "scene_material_image"
    ) {
        let subject_count = generation_spec_entity_ref::Entity::find()
            .filter(generation_spec_entity_ref::Column::GenerationSpecId.eq(spec.id))
            .filter(generation_spec_entity_ref::Column::Role.eq("subject"))
            .count(db)
            .await
            .map_err(AppError::Database)?;
        if subject_count == 0 {
            return Err(AppError::InvalidArgument(format!(
                "素材生成意图 {} 缺少 subject 实体引用",
                spec.id
            )));
        }
    }

    if let Some(required_roles) = spec
        .params_json
        .get("required_media_roles")
        .and_then(Value::as_array)
    {
        for role in required_roles {
            let role = role.as_str().ok_or_else(|| {
                AppError::InvalidArgument(
                    "params_json.required_media_roles 必须是字符串数组".to_string(),
                )
            })?;
            normalize_media_ref_role(role)?;
            let count = generation_spec_media_ref::Entity::find()
                .filter(generation_spec_media_ref::Column::GenerationSpecId.eq(spec.id))
                .filter(generation_spec_media_ref::Column::Role.eq(role))
                .count(db)
                .await
                .map_err(AppError::Database)?;
            if count == 0 {
                return Err(AppError::InvalidArgument(format!(
                    "生成意图 {} 缺少必需媒体引用: {}",
                    spec.id, role
                )));
            }
        }
    }
    Ok(())
}

pub(crate) fn render_prompt_text(
    spec: &generation_spec::Model,
    prompt_kind: &str,
) -> Result<String, AppError> {
    match prompt_kind {
        "image_prompt" => Ok(required_json_string(&spec.params_json, "image_prompt")?.to_string()),
        "negative_prompt" => {
            Ok(required_json_string(&spec.params_json, "negative_prompt")?.to_string())
        }
        "video_prompt" | "chat_video_prompt" | "provider_video_prompt" => {
            let input_images = required_json_string(&spec.params_json, "input_image_introduction")?;
            let segment_context = required_json_string(&spec.params_json, "segment_context")?;
            let time_distribution = required_time_distribution(&spec.params_json)?;
            let visual_style = required_json_string(&spec.params_json, "visual_style")?;
            let continuity_and_end_state =
                required_json_string(&spec.params_json, "continuity_and_end_state")?;
            let restrictions_and_avoidance =
                required_json_string(&spec.params_json, "restrictions_and_avoidance")?;
            Ok(format!(
                "[主体定义与素材绑定]\n{}\n\n[片段内容以及前因后果]\n{}\n\n[镜头分镜]\n{}\n\n[画面风格描述]\n{}\n\n[连续性与结尾状态]\n{}\n\n[限制与避免项]\n{}",
                input_images,
                segment_context,
                time_distribution,
                visual_style,
                continuity_and_end_state,
                restrictions_and_avoidance
            ))
        }
        _ => Err(AppError::InvalidArgument(format!(
            "提示词类型无效: {prompt_kind}"
        ))),
    }
}

fn required_time_distribution(params: &Value) -> Result<String, AppError> {
    let items = params
        .get("time_distribution")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AppError::InvalidArgument(
                "params_json.time_distribution 必须是非空字符串数组".to_string(),
            )
        })?;
    if items.is_empty() {
        return Err(AppError::InvalidArgument(
            "params_json.time_distribution 必须是非空字符串数组".to_string(),
        ));
    }
    let mut lines = Vec::with_capacity(items.len());
    for item in items {
        let line = item.as_str().ok_or_else(|| {
            AppError::InvalidArgument(
                "params_json.time_distribution 必须是非空字符串数组".to_string(),
            )
        })?;
        lines.push(required_text(line, "params_json.time_distribution[]")?.to_string());
    }
    Ok(lines.join("\n"))
}

fn validate_final_prompt_text(prompt_text: &str) -> Result<(), AppError> {
    required_text(prompt_text, "prompt_text")?;
    for token in INTERNAL_PROMPT_TOKENS {
        if prompt_text.contains(token) {
            return Err(AppError::InvalidArgument(format!(
                "最终提示词不能包含内部字段: {token}"
            )));
        }
    }
    Ok(())
}

pub(crate) fn validate_prompt_artifact_text(
    prompt_kind: &str,
    prompt_text: &str,
) -> Result<(), AppError> {
    validate_final_prompt_text(prompt_text)?;
    if matches!(
        normalize_prompt_kind(prompt_kind)?,
        "video_prompt" | "chat_video_prompt" | "provider_video_prompt"
    ) {
        for section in VIDEO_PROMPT_REQUIRED_SECTIONS {
            if !prompt_text.contains(section) {
                return Err(AppError::InvalidArgument(format!(
                    "视频提示词缺少必需章节 {section}"
                )));
            }
        }
    }
    Ok(())
}

pub(crate) async fn validate_entity_ref<C>(
    db: &C,
    root_project_id: i64,
    source_snapshot_id: i64,
    entity_kind: &str,
    entity_id: i64,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    match entity_kind {
        "character" | "object" | "scene" => {
            validate_snapshot_profile(db, source_snapshot_id, entity_kind, entity_id).await
        }
        "event" => Err(AppError::InvalidArgument(
            "事件档案尚未建立持久模型，不能作为生成意图实体引用".to_string(),
        )),
        _ => Err(AppError::InvalidArgument(format!(
            "实体类型无效: {entity_kind}"
        ))),
    }?;
    let exists_in_root = match entity_kind {
        "character" => protagonist_profile::Entity::find_by_id(entity_id)
            .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
            .count(db)
            .await
            .map_err(AppError::Database)?,
        "object" => object_profile::Entity::find_by_id(entity_id)
            .filter(object_profile::Column::ProjectId.eq(root_project_id))
            .count(db)
            .await
            .map_err(AppError::Database)?,
        "scene" => scene_profile::Entity::find_by_id(entity_id)
            .filter(scene_profile::Column::ProjectId.eq(root_project_id))
            .count(db)
            .await
            .map_err(AppError::Database)?,
        _ => 0,
    };
    if exists_in_root == 0 {
        return Err(AppError::InvalidArgument(format!(
            "实体引用 {}:{} 不属于根文本项目 {}",
            entity_kind, entity_id, root_project_id
        )));
    }
    Ok(())
}

async fn validate_snapshot_profile<C>(
    db: &C,
    snapshot_id: i64,
    profile_kind: &str,
    profile_id: i64,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let count = source_snapshot_profile::Entity::find()
        .filter(source_snapshot_profile::Column::SnapshotId.eq(snapshot_id))
        .filter(source_snapshot_profile::Column::ProfileKind.eq(profile_kind))
        .filter(source_snapshot_profile::Column::SourceProfileId.eq(profile_id))
        .count(db)
        .await
        .map_err(AppError::Database)?;
    if count == 0 {
        return Err(AppError::InvalidArgument(format!(
            "源文本快照 {} 中不存在 {} 档案 {}",
            snapshot_id, profile_kind, profile_id
        )));
    }
    Ok(())
}

fn required_json_string<'a>(params: &'a Value, field: &str) -> Result<&'a str, AppError> {
    let text = params.get(field).and_then(Value::as_str).ok_or_else(|| {
        AppError::InvalidArgument(format!("params_json.{field} 必须是非空字符串"))
    })?;
    required_text(text, &format!("params_json.{field}"))
}

fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!("{label} 不能为空")))
    } else {
        Ok(value)
    }
}

fn required_owned_text(value: String, label: &str) -> Result<String, AppError> {
    Ok(required_text(&value, label)?.to_string())
}

fn normalize_optional_owned_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_string();
        (!value.is_empty()).then_some(value)
    })
}

fn validate_positive_id(value: i64, label: &str) -> Result<(), AppError> {
    if value > 0 {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!("{label} 必须为正数")))
    }
}

pub(crate) fn normalize_entity_kind(value: &str) -> Result<&str, AppError> {
    normalize_one_of(value, "entity_kind", GENERATION_ENTITY_KINDS)
}

pub(crate) fn normalize_text_anchor_role(value: &str) -> Result<&str, AppError> {
    normalize_one_of(value, "text_anchor_role", GENERATION_TEXT_ANCHOR_ROLES)
}

pub(crate) fn normalize_entity_role(value: &str) -> Result<&str, AppError> {
    normalize_one_of(value, "entity_role", GENERATION_ENTITY_ROLES)
}

pub(crate) fn normalize_media_ref_role(value: &str) -> Result<&str, AppError> {
    normalize_one_of(value, "media_ref_role", GENERATION_MEDIA_REF_ROLES)
}

fn normalize_prompt_kind(value: &str) -> Result<&str, AppError> {
    normalize_one_of(value, "prompt_kind", GENERATION_PROMPT_KINDS)
}

pub(crate) fn normalize_video_prompt_kind(value: &str) -> Result<&str, AppError> {
    let value = normalize_prompt_kind(value)?;
    if matches!(
        value,
        "video_prompt" | "chat_video_prompt" | "provider_video_prompt"
    ) {
        Ok(value)
    } else {
        Err(AppError::InvalidArgument(format!(
            "prompt_kind 无效: {value}"
        )))
    }
}

fn normalize_one_of<'a>(
    value: &'a str,
    label: &str,
    allowed: &[&str],
) -> Result<&'a str, AppError> {
    let value = value.trim();
    if allowed.contains(&value) {
        Ok(value)
    } else {
        Err(AppError::InvalidArgument(format!("{label} 无效: {value}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn spec_with_params(params_json: Value) -> generation_spec::Model {
        let now = Utc::now().naive_utc();
        generation_spec::Model {
            id: 1,
            project_id: 1,
            root_project_id: 1,
            source_context_kind: Some("snapshot_backed".to_string()),
            source_snapshot_id: Some(1),
            source_chapter_id: Some(1),
            source_snapshot_chapter_id: Some(1),
            canvas_text_asset_id: None,
            target_kind: "video_segment".to_string(),
            target_entity_kind: "segment".to_string(),
            target_entity_id: Some(1),
            status: SPEC_STATUS_READY.to_string(),
            source_version_hash: "source-hash".to_string(),
            params_json,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn render_video_prompt_text_uses_current_main_sections() {
        let spec = spec_with_params(json!({
            "input_image_introduction": "从已有首帧自然开始。",
            "segment_context": "角色承接上一段尾态完成一次转身。",
            "time_distribution": ["0-2秒：角色停顿。", "2-5秒：角色转身并稳定。"],
            "visual_style": "清透动画风格，侧逆光。",
            "continuity_and_end_state": "结尾保持同一服装、站位和光源方向。",
            "restrictions_and_avoidance": "不要配背景音乐，不要字幕、水印、空间跳变。"
        }));

        let prompt = render_prompt_text(&spec, "video_prompt")
            .expect("video prompt should render from current schema");

        for section in VIDEO_PROMPT_REQUIRED_SECTIONS {
            assert!(prompt.contains(section), "missing section {section}");
        }
        assert!(!prompt.contains("[声音与对白]"));
        validate_prompt_artifact_text("video_prompt", &prompt)
            .expect("rendered video prompt should pass artifact validation");
    }

    #[test]
    fn render_video_prompt_text_rejects_old_sound_only_schema() {
        let spec = spec_with_params(json!({
            "input_image_introduction": "从已有首帧自然开始。",
            "segment_context": "角色承接上一段尾态完成一次转身。",
            "time_distribution": ["0-2秒：角色停顿。", "2-5秒：角色转身并稳定。"],
            "visual_style": "清透动画风格，侧逆光。",
            "sound_and_dialogue": "无对白。"
        }));

        let err = render_prompt_text(&spec, "video_prompt")
            .expect_err("old video prompt schema must fail explicitly");

        assert!(err.to_string().contains("continuity_and_end_state"));
    }

    #[test]
    fn validate_prompt_artifact_text_rejects_video_without_required_sections() {
        let err = validate_prompt_artifact_text(
            "provider_video_prompt",
            "[主体定义与素材绑定]\n只有图片介绍，没有完整视频主提示词章节。",
        )
        .expect_err("video artifact text must include the current main sections");

        assert!(err.to_string().contains("[片段内容以及前因后果]"));
    }

    #[test]
    fn validate_prompt_artifact_text_allows_image_prompt_without_video_sections() {
        validate_prompt_artifact_text("image_prompt", "清晰写实的单张角色立绘。")
            .expect("image prompt should not require video prompt sections");
    }

    #[test]
    fn append_generation_artifact_target_requires_current_targets_shape() {
        let target = GenerationRunTraceTarget {
            target_kind: "generated_image".to_string(),
            target_table: "generated_image".to_string(),
            target_id: 9,
        };

        let refs = append_generation_artifact_target(None, target.clone())
            .expect("empty artifact refs should create current targets shape");
        assert_eq!(refs["targets"].as_array().map(Vec::len), Some(1));

        let refs = append_generation_artifact_target(
            Some(json!({
                "source": "checkpoint",
                "targets": [{
                    "target_kind": "chapter_video_prompt",
                    "target_table": "chapter_video_prompt",
                    "target_id": 7,
                }],
            })),
            target.clone(),
        )
        .expect("current artifact refs shape should append target");
        assert_eq!(refs["source"], "checkpoint");
        assert_eq!(refs["targets"].as_array().map(Vec::len), Some(2));

        let err = append_generation_artifact_target(Some(json!(["legacy-target"])), target.clone())
            .expect_err("non-object artifact refs must fail explicitly");
        assert!(err.to_string().contains("artifact_refs_json 必须是对象"));

        let err = append_generation_artifact_target(
            Some(json!({
                "targets": {
                    "target_kind": "legacy-shape",
                },
            })),
            target,
        )
        .expect_err("non-array targets must fail explicitly");
        assert!(
            err.to_string()
                .contains("artifact_refs_json.targets 必须是数组")
        );
    }
}
