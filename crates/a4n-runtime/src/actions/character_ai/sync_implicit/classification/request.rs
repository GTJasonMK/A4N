use serde_json::{Value, json};

use crate::actions::character_ai::implicit_classification::{
    build_implicit_classification_manifest_user_prompt, implicit_classification_stable_context,
};
use crate::actions::llm_request::{
    ContextAwareJsonLlmRequestInput, PACK_KIND_IMPLICIT_CLASSIFICATION_CONTEXT,
    context_aware_system_user_json_llm_request,
};
use crate::actions::workflow_checkpoint::{
    llm_call_checkpoint_fingerprint_sha256, required_llm_call_workflow_run_id,
};
use a4n_core::config::app_config::llm_token_profiles;
use a4n_llm::provider::ChatRequest;
use a4n_llm::trace::context_aware_chat_request_trace_json;

pub(super) struct PreparedImplicitClassificationCall {
    pub(super) request: ChatRequest,
    pub(super) call_key: String,
    pub(super) workflow_run_id: i64,
    pub(super) input_fingerprint_json: Value,
}

pub(super) fn prepare_implicit_classification_call(
    project_id: i64,
    workflow_run_id: Option<i64>,
    profile_name: &str,
    behavior_index: usize,
    chapter: i32,
    behavior: &Value,
    implicit_attributes: &Value,
    system_prompt: &str,
) -> Result<PreparedImplicitClassificationCall, String> {
    let user_prompt = build_implicit_classification_manifest_user_prompt(profile_name, behavior)?;
    let stable_context = implicit_classification_stable_context(implicit_attributes)?;
    let call_key = format!("implicit_classification:{profile_name}:{chapter}:{behavior_index}");
    let workflow_run_id = required_llm_call_workflow_run_id(workflow_run_id, &call_key)
        .map_err(|err| err.to_string())?;
    let mut request = context_aware_system_user_json_llm_request(
        system_prompt.to_string(),
        ContextAwareJsonLlmRequestInput {
            project_id,
            workflow_run_id: Some(workflow_run_id),
            workflow: "implicit_classification".to_string(),
            step_key: "behavior_classification".to_string(),
            scope_kind: "profile_sync".to_string(),
            pack_kind: PACK_KIND_IMPLICIT_CLASSIFICATION_CONTEXT.to_string(),
            source_refs_json: Some(json!({
                "profile_name": profile_name,
                "behavior_index": behavior_index,
                "chapter_number": chapter})),
            config_json: json!({
                "ledger_version": "implicit_classification_context_v1",
                "prompt_name": "implicit_classification"}),
            stable_context_json: stable_context,
            dynamic_user_prompt: user_prompt,
            temperature: Some(0.2),
            max_tokens_profile: llm_token_profiles::JSON_STANDARD.to_string(),
            call_key: Some(call_key.clone()),
        },
    )?;
    let request_json = context_aware_chat_request_trace_json(&request)
        .map_err(|err| format!("行为隐性属性分类 LLM 请求序列化失败: {err}"))?;
    let input_fingerprint_json = json!({
        "request": request_json,
        "profile_name": profile_name,
        "behavior_index": behavior_index,
        "chapter_number": chapter,
        "implicit_attributes": implicit_attributes,
        "behavior": behavior});
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
        .map_err(|err| err.to_string())?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }

    Ok(PreparedImplicitClassificationCall {
        request,
        call_key,
        workflow_run_id,
        input_fingerprint_json,
    })
}
