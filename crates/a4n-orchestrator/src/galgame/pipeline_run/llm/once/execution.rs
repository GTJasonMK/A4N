use a4n_core::error::AppError;
use a4n_db::workflow::{self, CompletedLlmCallCheckpointInput};
use a4n_llm::provider::{ChatMessage, ChatRequest, LlmTraceContext, is_truncated_finish_reason};
use serde_json::{Value, json};

use crate::galgame::{OrchestratorContext, pipeline_json::galgame_llm_output_schema};
use crate::structured_llm::{StructuredRepairCheckpointContext, StructuredValueRepairInput};
use crate::workflow::workflow_write_app_error;

use super::super::super::contracts::GALGAME_PIPELINE_MAX_TOKENS_PROFILE;
use super::{GalgameLlmOnceInput, PreparedGalgameLlmOnce};

pub(super) async fn execute_galgame_llm_once<F>(
    service: &OrchestratorContext,
    input: &GalgameLlmOnceInput<'_>,
    prepared: PreparedGalgameLlmOnce,
    validate: F,
) -> Result<Value, AppError>
where
    F: Fn(Value) -> Result<Value, AppError>,
{
    let request = ChatRequest::new(
        vec![
            ChatMessage::system(prepared.system_prompt),
            ChatMessage::user(input.user_prompt.clone()),
        ],
        Some(0.25),
        None,
        Some(GALGAME_PIPELINE_MAX_TOKENS_PROFILE.to_string()),
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
    let response = service.stream_complete_llm(request.clone()).await?;
    if is_truncated_finish_reason(response.finish_reason.as_deref()) {
        return Err(AppError::InvalidState {
            current: format!("{}_response_truncated", input.prompt_name),
            operation: "执行 Galgame source sync 局部 LLM 重跑".to_string(),
        });
    }
    let raw_output = response.message.content;
    let parse_output = |value: &Value| validate(value.clone());
    let repaired = service
        .parse_validate_or_repair_structured_value(StructuredValueRepairInput {
            request,
            raw_response: &raw_output,
            label: input.prompt_name,
            schema: galgame_llm_output_schema(input.prompt_name)?,
            checkpoint: Some(StructuredRepairCheckpointContext {
                workflow_run_id: input.workflow_run_id,
                base_call_key: input.step_key.clone(),
                call_order: input.call_order,
            }),
            cancel_token: None,
            progress: None,
            parse_output: &parse_output,
        })
        .await?;
    let repair_artifact = repaired.artifact_summary();
    let parsed_output = repaired.output;
    let usage_json = response
        .usage
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(AppError::Serialization)?;
    workflow::save_completed_llm_call_checkpoint(
        service.database.connection(),
        CompletedLlmCallCheckpointInput {
            workflow_run_id: input.workflow_run_id,
            call_key: input.step_key.clone(),
            call_order: input.call_order,
            input_fingerprint_json: prepared.fingerprint_json,
            checkpoint_json: json!({
                "output": parsed_output.clone(),
                "parsed_output": parsed_output.clone(),
                "response_content": raw_output,
                "usage": usage_json,
                "finish_reason": response.finish_reason,
            }),
            artifacts_json: Some(json!({
                "workflow": input.workflow,
                "prompt_name": input.prompt_name,
                "structured_repair": repair_artifact,
            })),
        },
    )
    .await
    .map_err(workflow_write_app_error)?;
    Ok(parsed_output)
}
