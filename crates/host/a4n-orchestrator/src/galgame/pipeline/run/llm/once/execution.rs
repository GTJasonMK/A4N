use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{
    ChatMessage, ChatRequest, LlmTraceContext, is_truncated_finish_reason,
};
use serde_json::{Value, json};

use crate::galgame::OrchestratorContext;
use crate::structured_llm::parse_strict_json_response;
use crate::workflow::{CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput};

use super::{GalgameLlmOnceInput, PreparedGalgameLlmOnce};

pub(super) async fn execute_galgame_llm_once<T, F>(
    service: &OrchestratorContext,
    input: &GalgameLlmOnceInput<'_>,
    prepared: PreparedGalgameLlmOnce,
    validate: F,
) -> Result<(Value, T), AppError>
where
    T: serde::Serialize,
    F: Fn(Value) -> Result<T, AppError>,
{
    let request = ChatRequest::new(
        vec![
            ChatMessage::system(prepared.system_prompt),
            ChatMessage::user(input.user_prompt.clone()),
        ],
        Some(0.25),
        None,
    )
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(input.project_id),
        root_project_id: Some(input.root_project_id),
        workflow: input.workflow.to_string(),
        step_key: input.step_key.clone(),
        generation_run_id: None,
        workflow_run_id: Some(input.workflow_run_id),
        call_key: Some(input.step_key.clone()),
        input_fingerprint_sha256: Some(prepared.input_fingerprint_sha256),
        context_plan_json: None,
        context_pack_refs_json: None,
    });
    let response = service.stream_complete_llm(request).await?;
    if is_truncated_finish_reason(response.finish_reason.as_deref()) {
        return Err(AppError::InvalidState {
            current: format!("{}_response_truncated", input.prompt_name),
            operation: "执行 Galgame source sync 局部 LLM 重跑".to_string(),
        });
    }
    let raw_output = response.message.content;
    let parsed_output = validate(parse_strict_json_response(&raw_output, input.prompt_name)?)?;
    let parsed_json = serde_json::to_value(&parsed_output)?;
    let usage_json = response
        .usage
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(AppError::Serialization)?;
    service
        .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id: input.workflow_run_id,
            call_key: input.step_key.clone(),
            call_order: input.call_order,
            input_fingerprint_json: prepared.fingerprint_json,
            checkpoint: CompletedLlmCallCheckpoint {
                parsed_output: parsed_json.clone(),
                response_content: raw_output,
                usage: usage_json,
                finish_reason: response.finish_reason,
                artifacts_json: Some(json!({
                    "workflow": input.workflow,
                    "prompt_name": input.prompt_name,
                })),
            },
        })
        .await?;
    Ok((parsed_json, parsed_output))
}
