use a4n_engine::llm::provider::{ChatRequest, LlmTraceContext};
use a4n_services::llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, context_aware_system_user_request,
};
use a4n_services::source::source_content_sha256;
use serde_json::{Value, json};

pub(crate) const PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT: &str = "profile_sync_source_context";
pub(crate) const PACK_KIND_SCENE_PROFILE_SYNC_SOURCE_CONTEXT: &str =
    "scene_profile_sync_source_context";
pub(crate) const PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT: &str =
    "character_conflict_source_context";
pub(crate) const PACK_KIND_CHAPTER_SCENE_GENERATION_CONTEXT: &str =
    "chapter_scene_generation_context";
pub(crate) const PACK_KIND_IMPLICIT_CLASSIFICATION_CONTEXT: &str =
    "implicit_classification_context";
pub(crate) const PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT: &str = "implicit_update_review_context";

#[derive(Debug, Clone)]
pub(crate) struct ContextAwareJsonLlmRequestInput {
    pub(crate) project_id: i64,
    pub(crate) workflow_run_id: Option<i64>,
    pub(crate) workflow: String,
    pub(crate) step_key: String,
    pub(crate) scope_kind: String,
    pub(crate) pack_kind: String,
    pub(crate) source_refs_json: Option<Value>,
    pub(crate) config_json: Value,
    pub(crate) stable_context_json: Value,
    pub(crate) dynamic_user_prompt: String,
    pub(crate) temperature: Option<f32>,
    pub(crate) max_tokens_profile: String,
    pub(crate) call_key: Option<String>,
}

pub(crate) fn context_aware_system_user_llm_request(
    system_prompt: impl Into<String>,
    input: ContextAwareJsonLlmRequestInput,
) -> Result<ChatRequest, String> {
    let stable_context_sha256 = source_content_sha256(
        &serde_json::to_string(&input.stable_context_json)
            .map_err(|err| format!("LLM 稳定上下文序列化失败: {err}"))?,
    );
    let dynamic_user_sha256 = source_content_sha256(&input.dynamic_user_prompt);
    let mut source_refs = input.source_refs_json.unwrap_or_else(|| json!({}));
    if let Some(map) = source_refs.as_object_mut() {
        map.insert(
            "stable_context_sha256".to_string(),
            json!(stable_context_sha256.clone()),
        );
    }
    let mut ledger = LlmContextLedger::new();
    ledger
        .add_pack(a4n_services::llm_context::LlmContextLedgerPackInput {
            scope_kind: (&input.scope_kind).into(),
            pack_kind: (&input.pack_kind).into(),
            source_refs_json: Some(source_refs),
            config_identity_json: input.config_json,
            required: true,
            stable_prefix: true,
            variants: vec![
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_FULL_RAW,
                    input.stable_context_json.clone(),
                ),
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_FOCUSED_RAW,
                    input.stable_context_json.clone(),
                ),
                LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, input.stable_context_json),
            ],
        })
        .map_err(|err| err.to_string())?;
    let manifest = ledger.into_manifest(
        input.dynamic_user_prompt,
        Some(
            "只返回当前任务要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段。"
                .to_string(),
        ),
    );
    let fingerprint = source_content_sha256(
        &serde_json::to_string(&json!({
            "workflow": &input.workflow,
            "step_key": &input.step_key,
            "stable_context_sha256": stable_context_sha256,
            "dynamic_user_sha256": dynamic_user_sha256,
            "max_tokens_profile": &input.max_tokens_profile,
            "temperature": input.temperature,
        }))
        .map_err(|err| format!("LLM 请求 fingerprint 序列化失败: {err}"))?,
    );
    Ok(context_aware_system_user_request(
        system_prompt,
        manifest,
        input.temperature,
        None,
        Some(input.max_tokens_profile),
    )
    .map_err(|err| err.to_string())?
    .with_trace_context(LlmTraceContext {
        project_id: Some(input.project_id),
        root_project_id: None,
        workflow: input.workflow,
        step_key: input.step_key,
        generation_run_id: None,
        workflow_run_id: input.workflow_run_id,
        call_key: input.call_key,
        input_fingerprint_sha256: Some(fingerprint),
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}

pub(crate) fn context_aware_system_user_json_llm_request(
    system_prompt: impl Into<String>,
    input: ContextAwareJsonLlmRequestInput,
) -> Result<ChatRequest, String> {
    Ok(context_aware_system_user_llm_request(system_prompt, input)?.with_json_response_format())
}

pub(crate) fn chat_request_with_workflow_run_id(
    mut request: ChatRequest,
    workflow_run_id: i64,
) -> ChatRequest {
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
    }
    request
}
