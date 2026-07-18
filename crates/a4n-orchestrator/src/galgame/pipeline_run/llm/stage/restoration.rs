use a4n_core::error::AppError;
use a4n_db::workflow;
use serde_json::Value;

use crate::galgame::OrchestratorContext;
use crate::workflow::{workflow_checkpoint_contract_app_error, workflow_read_app_error};

use super::super::super::checkpoints::restored_galgame_llm_stage_output;
use super::{GalgameLlmStageInput, PreparedGalgameLlmStage};

pub(super) async fn restore_galgame_llm_stage<F>(
    service: &OrchestratorContext,
    input: &GalgameLlmStageInput<'_>,
    prepared: &PreparedGalgameLlmStage,
    validate: &F,
) -> Result<Option<Value>, AppError>
where
    F: Fn(Value) -> Result<Value, AppError>,
{
    if !input.restore_from_checkpoint {
        return Ok(None);
    }
    if let Some(checkpoint) = workflow::load_completed_llm_call_checkpoint(
        service.db.connection(),
        input.workflow_run_id,
        input.checkpoint_key,
        &prepared.fingerprint_json,
    )
    .await
    .map_err(workflow_read_app_error)?
    {
        let checkpoint_json =
            workflow::required_llm_call_checkpoint_json(checkpoint, input.checkpoint_key)
                .map_err(workflow_checkpoint_contract_app_error)?;
        let restored = restored_galgame_llm_stage_output(
            &checkpoint_json,
            input.checkpoint_key,
            input.prompt_name,
            |value| validate(value),
        )?;
        return Ok(Some(restored));
    }
    if input.restore_required {
        return Err(AppError::InvalidState {
            current: format!("{}_checkpoint_missing", input.checkpoint_key),
            operation: format!("从 {} 阶段重启 Galgame 管线", input.stage.key()),
        });
    }
    Ok(None)
}
