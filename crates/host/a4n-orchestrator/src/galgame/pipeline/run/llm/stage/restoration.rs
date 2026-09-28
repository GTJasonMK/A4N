use a4n_error::AppError;
use serde_json::Value;

use crate::galgame::OrchestratorContext;

use super::{GalgameLlmStageInput, PreparedGalgameLlmStage};
use crate::structured_llm::restored_llm_stage_output as restored_galgame_llm_stage_output;

pub(super) async fn restore_galgame_llm_stage<T, F>(
    service: &OrchestratorContext,
    input: &GalgameLlmStageInput<'_>,
    prepared: &PreparedGalgameLlmStage,
    validate: &F,
) -> Result<Option<(Value, T)>, AppError>
where
    T: serde::Serialize,
    F: Fn(Value) -> Result<T, AppError>,
{
    if !input.restore_from_checkpoint {
        return Ok(None);
    }
    if let Some(checkpoint) = service
        .load_completed_llm_call_checkpoint(
            input.workflow_run_id,
            input.checkpoint_key,
            &prepared.fingerprint_json,
        )
        .await?
    {
        let restored = restored_galgame_llm_stage_output(
            &checkpoint.parsed_output,
            &checkpoint.response_content,
            input.checkpoint_key,
            input.prompt_name,
            validate,
        );
        let restored = service
            .validate_llm_call_output(
                input.workflow_run_id,
                input.checkpoint_key,
                &prepared.fingerprint_json,
                restored,
            )
            .await?;
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
