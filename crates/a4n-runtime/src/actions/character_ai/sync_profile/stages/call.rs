use a4n_llm::{provider::ChatRequest, trace::context_aware_chat_request_trace_json};
use serde_json::{Value, json};

use crate::actions::character_ai::sync_llm::ensure_character_sync_response_not_truncated;
use crate::actions::character_ai::sync_types::CharacterSyncContext;
use crate::actions::workflow_checkpoint::{
    llm_call_checkpoint_fingerprint_sha256, required_llm_call_checkpoint_json,
    required_llm_call_workflow_run_id,
};
use crate::state::AppState;

use super::super::contracts::CharacterSyncStage;
use super::super::request::build_character_sync_stage_request;
use super::rules::{
    optional_character_sync_checkpoint_string, required_character_sync_checkpoint_string,
};

pub(super) struct CharacterSyncStageCall {
    pub(super) workflow_run_id: i64,
    pub(super) call_key: String,
    pub(super) request: ChatRequest,
    pub(super) input_fingerprint_json: Value,
    pub(super) response_content: String,
    pub(super) response_usage_json: Option<Value>,
    pub(super) response_finish_reason: Option<String>,
    pub(super) restored_parsed_output: Option<Value>,
}

pub(super) async fn run_character_sync_stage_call(
    state: &AppState,
    project_id: i64,
    workflow_run_id: Option<i64>,
    context: &CharacterSyncContext,
    stage: CharacterSyncStage,
    previous_steps: &Value,
) -> Result<CharacterSyncStageCall, String> {
    let mut request =
        build_character_sync_stage_request(state, project_id, context, stage, previous_steps)
            .await?;
    let call_key = format!(
        "character_sync:{}:{}:{}",
        context.profile.id,
        context.target_chapter,
        stage.key()
    );
    let workflow_run_id = required_llm_call_workflow_run_id(workflow_run_id, &call_key)
        .map_err(|err| err.to_string())?;
    let request_json = context_aware_chat_request_trace_json(&request)
        .map_err(|err| format!("角色同步阶段 LLM 请求序列化失败: {err}"))?;
    let input_fingerprint_json = json!({
        "request": request_json,
        "stage": stage.key(),
        "profile_id": context.profile.id,
        "start_chapter": context.start_chapter,
        "target_chapter": context.target_chapter,
        "previous_steps": previous_steps});
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
        .map_err(|err| err.to_string())?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    state
        .orchestrator
        .mark_workflow_current_step(workflow_run_id, &call_key)
        .await
        .map_err(|err| err.to_string())?;

    let (response_content, response_usage_json, response_finish_reason, restored_parsed_output) =
        if let Some(checkpoint) = state
            .load_completed_llm_call_checkpoint(workflow_run_id, &call_key, &input_fingerprint_json)
            .await
            .map_err(|err| err.to_string())?
        {
            let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)
                .map_err(|err| err.to_string())?;
            let parsed_output = checkpoint_json
                .get("parsed_output")
                .cloned()
                .ok_or_else(|| format!("{call_key}.parsed_output 缺失"))?;
            let response_content = required_character_sync_checkpoint_string(
                &checkpoint_json,
                &call_key,
                "response_content",
            )?;
            let finish_reason =
                optional_character_sync_checkpoint_string(&checkpoint_json, "finish_reason")?;
            ensure_character_sync_response_not_truncated(
                finish_reason.as_deref(),
                &format!("{} {}", context.profile.name, stage.title()),
            )?;
            let usage = checkpoint_json
                .get("usage")
                .filter(|value| !value.is_null())
                .cloned();
            (response_content, usage, finish_reason, Some(parsed_output))
        } else {
            let response = state
                .llm
                .chat(request.clone())
                .await
                .map_err(|err| err.to_string())?;
            ensure_character_sync_response_not_truncated(
                response.finish_reason.as_deref(),
                &format!("{} {}", context.profile.name, stage.title()),
            )?;
            let response_usage_json = response
                .usage
                .as_ref()
                .map(serde_json::to_value)
                .transpose()
                .map_err(|err| format!("角色同步阶段 LLM 用量序列化失败: {err}"))?;
            (
                response.message.content,
                response_usage_json,
                response.finish_reason,
                None,
            )
        };

    Ok(CharacterSyncStageCall {
        workflow_run_id,
        call_key,
        request,
        input_fingerprint_json,
        response_content,
        response_usage_json,
        response_finish_reason,
        restored_parsed_output,
    })
}
