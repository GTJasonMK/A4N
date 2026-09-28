use a4n_error::AppError;
use serde_json::Value;

use crate::OrchestratorContext;

use super::{PreparedWorkflowLlmOnce, WorkflowLlmOnceInput};
use crate::structured_llm::restored_llm_stage_output;

pub(super) async fn restore_workflow_llm_once<T, F>(
    service: &OrchestratorContext,
    input: &WorkflowLlmOnceInput<'_>,
    prepared: &PreparedWorkflowLlmOnce,
    validate: &F,
) -> Result<Option<(Value, T)>, AppError>
where
    T: serde::Serialize,
    F: Fn(Value) -> Result<T, AppError>,
{
    let Some(checkpoint) = service
        .load_completed_llm_call_checkpoint(
            input.workflow_run_id,
            &input.step_key,
            &prepared.fingerprint_json,
        )
        .await?
    else {
        return Ok(None);
    };
    let restored = restored_llm_stage_output(
        &checkpoint.parsed_output,
        &checkpoint.response_content,
        &input.step_key,
        input.prompt_name,
        validate,
    );
    let restored = service
        .validate_llm_call_output(
            input.workflow_run_id,
            &input.step_key,
            &prepared.fingerprint_json,
            restored,
        )
        .await?;
    Ok(Some(restored))
}
