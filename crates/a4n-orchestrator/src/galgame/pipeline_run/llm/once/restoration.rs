use a4n_error::AppError;
use serde_json::Value;

use crate::galgame::OrchestratorContext;

use super::super::super::checkpoints::restored_galgame_llm_stage_output;
use super::{GalgameLlmOnceInput, PreparedGalgameLlmOnce};

pub(super) async fn restore_galgame_llm_once<F>(
    service: &OrchestratorContext,
    input: &GalgameLlmOnceInput<'_>,
    prepared: &PreparedGalgameLlmOnce,
    validate: &F,
) -> Result<Option<Value>, AppError>
where
    F: Fn(Value) -> Result<Value, AppError>,
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
    let restored = restored_galgame_llm_stage_output(
        &checkpoint.parsed_output,
        &checkpoint.response_content,
        &input.step_key,
        input.prompt_name,
        validate,
    )?;
    Ok(Some(restored))
}
