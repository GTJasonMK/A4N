use a4n_domain::source_text_hash;
use serde_json::{Value, json};

use super::super::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, LlmContextLedgerPackInput,
};
use super::context_aware_system_user_request;
use crate::provider::{ChatRequest, LlmTraceContext};

#[derive(Debug, Clone)]
pub struct ContextAwareJsonLlmRequestInput {
    pub project_id: i64,
    pub workflow_run_id: Option<i64>,
    pub workflow: String,
    pub step_key: String,
    pub scope_kind: String,
    pub pack_kind: String,
    pub source_refs_json: Option<Value>,
    pub config_json: Value,
    pub stable_context_json: Value,
    pub dynamic_user_prompt: String,
    pub temperature: Option<f32>,
    pub max_tokens_profile: String,
    pub call_key: Option<String>,
}

pub fn context_aware_system_user_llm_request(
    system_prompt: impl Into<String>,
    input: ContextAwareJsonLlmRequestInput,
) -> Result<ChatRequest, String> {
    let stable_context_sha256 = source_text_hash(
        &serde_json::to_string(&input.stable_context_json)
            .map_err(|error| format!("LLM 稳定上下文序列化失败: {error}"))?,
    );
    let dynamic_user_sha256 = source_text_hash(&input.dynamic_user_prompt);
    let mut source_refs = input.source_refs_json.unwrap_or_else(|| json!({}));
    if let Some(map) = source_refs.as_object_mut() {
        map.insert(
            "stable_context_sha256".to_string(),
            json!(stable_context_sha256),
        );
    }

    let mut ledger = LlmContextLedger::new();
    ledger
        .add_pack(LlmContextLedgerPackInput {
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
        .map_err(|error| error.to_string())?;
    let manifest = ledger.into_manifest(
        input.dynamic_user_prompt,
        Some(
            "只返回当前任务要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段。"
                .to_string(),
        ),
    );
    let fingerprint = source_text_hash(
        &serde_json::to_string(&json!({
            "workflow": &input.workflow,
            "step_key": &input.step_key,
            "stable_context_sha256": stable_context_sha256,
            "dynamic_user_sha256": dynamic_user_sha256,
            "max_tokens_profile": &input.max_tokens_profile,
            "temperature": input.temperature,
        }))
        .map_err(|error| format!("LLM 请求 fingerprint 序列化失败: {error}"))?,
    );

    Ok(context_aware_system_user_request(
        system_prompt,
        manifest,
        input.temperature,
        None,
        Some(input.max_tokens_profile),
    )
    .map_err(|error| error.to_string())?
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

pub fn context_aware_system_user_json_llm_request(
    system_prompt: impl Into<String>,
    input: ContextAwareJsonLlmRequestInput,
) -> Result<ChatRequest, String> {
    Ok(context_aware_system_user_llm_request(system_prompt, input)?.with_json_response_format())
}

pub fn chat_request_with_workflow_run_id(
    mut request: ChatRequest,
    workflow_run_id: i64,
) -> ChatRequest {
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
    }
    request
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_input() -> ContextAwareJsonLlmRequestInput {
        ContextAwareJsonLlmRequestInput {
            project_id: 7,
            workflow_run_id: Some(11),
            workflow: "profile_sync".to_string(),
            step_key: "extract".to_string(),
            scope_kind: "project".to_string(),
            pack_kind: "profile_sync_source_context".to_string(),
            source_refs_json: Some(json!({"chapter_ids": [3]})),
            config_json: json!({"schema_version": 1}),
            stable_context_json: json!({"chapter": {"id": 3, "content": "潮汐城"}}),
            dynamic_user_prompt: "提取角色档案".to_string(),
            temperature: Some(0.2),
            max_tokens_profile: "profile_sync".to_string(),
            call_key: Some("profile_sync:extract".to_string()),
        }
    }

    #[test]
    fn builds_context_manifest_fingerprint_and_trace() {
        let request = context_aware_system_user_llm_request("system", request_input())
            .expect("request should build");
        let manifest = request
            .context_manifest_json
            .as_ref()
            .expect("context manifest should exist");
        let trace = request
            .trace_context
            .as_ref()
            .expect("trace context should exist");

        assert_eq!(manifest["packs"][0]["scope_kind"], "project");
        assert_eq!(
            manifest["packs"][0]["pack_kind"],
            "profile_sync_source_context"
        );
        assert_eq!(
            manifest["packs"][0]["variants"].as_array().unwrap().len(),
            3
        );
        assert_eq!(trace.project_id, Some(7));
        assert_eq!(trace.workflow_run_id, Some(11));
        assert_eq!(trace.call_key.as_deref(), Some("profile_sync:extract"));
        assert_eq!(
            trace.input_fingerprint_sha256.as_deref().map(str::len),
            Some(64)
        );
    }

    #[test]
    fn json_builder_and_workflow_run_rebind_preserve_request_contract() {
        let request = context_aware_system_user_json_llm_request("system", request_input())
            .expect("JSON request should build");
        let request = chat_request_with_workflow_run_id(request, 29);

        assert!(request.response_format.is_some());
        assert_eq!(
            request
                .trace_context
                .as_ref()
                .and_then(|trace| trace.workflow_run_id),
            Some(29)
        );
    }
}
