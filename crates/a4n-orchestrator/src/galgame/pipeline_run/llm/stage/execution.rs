use a4n_core::error::AppError;
use a4n_llm::provider::{ChatRequest, LlmTraceContext, is_truncated_finish_reason};
use serde_json::{Value, json};

use crate::galgame::OrchestratorContext;
use crate::structured_llm::parse_strict_json_response;
use crate::workflow::{CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput};

use super::super::super::contracts::{
    GALGAME_PIPELINE_MAX_TOKENS_PROFILE, GALGAME_PIPELINE_WORKFLOW,
};
use super::super::super::progress::ensure_galgame_pipeline_not_cancelled;
use super::{GalgameLlmStageInput, PreparedGalgameLlmStage};

pub(super) async fn execute_galgame_llm_stage<F>(
    service: &OrchestratorContext,
    input: &GalgameLlmStageInput<'_>,
    prepared: PreparedGalgameLlmStage,
    validate: F,
) -> Result<Value, AppError>
where
    F: Fn(Value) -> Result<Value, AppError>,
{
    let request = ChatRequest::new(
        prepared.request_messages,
        Some(0.25),
        None,
        Some(GALGAME_PIPELINE_MAX_TOKENS_PROFILE.to_string()),
    )
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(input.project_id),
        root_project_id: Some(input.root_project_id),
        workflow: GALGAME_PIPELINE_WORKFLOW.to_string(),
        step_key: input.stage.key().to_string(),
        generation_run_id: None,
        workflow_run_id: Some(input.workflow_run_id),
        call_key: Some(input.checkpoint_key.to_string()),
        input_fingerprint_sha256: Some(prepared.input_fingerprint_sha256),
        context_plan_json: None,
        context_pack_refs_json: None,
    });
    let response = service.stream_complete_llm(request).await?;
    ensure_galgame_pipeline_not_cancelled(service, input.workflow_run_id).await?;
    if is_truncated_finish_reason(response.finish_reason.as_deref()) {
        return Err(AppError::InvalidState {
            current: format!("{}_response_truncated", input.prompt_name),
            operation: "执行 Galgame 管线 LLM 阶段".to_string(),
        });
    }
    let raw_output = response.message.content;
    let parsed_output = validate(parse_strict_json_response(&raw_output, input.prompt_name)?)?;
    ensure_galgame_pipeline_not_cancelled(service, input.workflow_run_id).await?;
    let usage_json = response
        .usage
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(AppError::Serialization)?;
    service
        .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id: input.workflow_run_id,
            call_key: input.checkpoint_key.to_string(),
            call_order: input.stage.order(),
            input_fingerprint_json: prepared.fingerprint_json,
            checkpoint: CompletedLlmCallCheckpoint {
                parsed_output: parsed_output.clone(),
                response_content: raw_output,
                usage: usage_json,
                finish_reason: response.finish_reason,
                artifacts_json: Some(json!({
                    "stage": input.stage.key(),
                    "prompt_name": input.prompt_name,
                })),
            },
        })
        .await?;
    Ok(parsed_output)
}
