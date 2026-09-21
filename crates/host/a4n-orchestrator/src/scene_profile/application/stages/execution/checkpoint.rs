use a4n_error::AppError;
use serde_json::Value;

use crate::OrchestratorContext;
use crate::workflow::{CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput};

use super::super::super::types::SceneProfileStageCheckpoint;

pub(super) async fn load_completed_scene_profile_stage_checkpoint(
    state: &OrchestratorContext,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<Option<SceneProfileStageCheckpoint>, AppError> {
    let Some(checkpoint) = state
        .load_completed_llm_call_checkpoint(workflow_run_id, call_key, input_fingerprint_json)
        .await?
    else {
        return Ok(None);
    };
    Ok(Some(SceneProfileStageCheckpoint {
        response_content: checkpoint.response_content,
        parsed_output: checkpoint.parsed_output,
    }))
}

pub(super) async fn save_completed_scene_profile_stage_checkpoint(
    state: &OrchestratorContext,
    workflow_run_id: i64,
    call_key: String,
    call_order: i32,
    input_fingerprint_json: Value,
    response_content: &str,
    parsed_output: Value,
) -> Result<(), AppError> {
    state
        .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key,
            call_order,
            input_fingerprint_json,
            checkpoint: CompletedLlmCallCheckpoint {
                parsed_output,
                response_content: response_content.to_string(),
                usage: None,
                finish_reason: None,
                artifacts_json: None,
            },
        })
        .await
}
