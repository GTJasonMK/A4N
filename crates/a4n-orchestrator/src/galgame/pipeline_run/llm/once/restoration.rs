use a4n_core::error::AppError;
use a4n_db::workflow;
use serde_json::Value;

use crate::galgame::OrchestratorContext;
use crate::workflow::{workflow_checkpoint_contract_app_error, workflow_read_app_error};

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
    let Some(checkpoint) = workflow::load_completed_llm_call_checkpoint(
        service.database.connection(),
        input.workflow_run_id,
        &input.step_key,
        &prepared.fingerprint_json,
    )
    .await
    .map_err(workflow_read_app_error)?
    else {
        return Ok(None);
    };
    let checkpoint_json = workflow::required_llm_call_checkpoint_json(checkpoint, &input.step_key)
        .map_err(workflow_checkpoint_contract_app_error)?;
    let restored = restored_galgame_llm_stage_output(
        &checkpoint_json,
        &input.step_key,
        input.prompt_name,
        |value| validate(value),
    )?;
    Ok(Some(restored))
}
