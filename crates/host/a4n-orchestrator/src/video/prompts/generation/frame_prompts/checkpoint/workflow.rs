use a4n_db::{
    generation::generation_checkpoint_fingerprint_sha256, workflow::UpsertWorkflowCheckpointInput,
};
use a4n_domain::workflow::WORKFLOW_STATUS_COMPLETED;
use a4n_error::AppError;
use serde_json::Value;

use super::super::OrchestratorContext;

pub(in crate::video::prompts::generation::frame_prompts) async fn save_completed_video_workflow_checkpoint(
    service: &OrchestratorContext,
    workflow_run_id: i64,
    step_key: String,
    step_order: i32,
    input_fingerprint_json: Value,
    checkpoint_json: Value,
    artifacts_json: Option<Value>,
) -> Result<(), AppError> {
    let input_fingerprint_sha256 =
        generation_checkpoint_fingerprint_sha256(&input_fingerprint_json)?;
    service
        .upsert_workflow_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id,
            step_key,
            step_order,
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            input_fingerprint_sha256: Some(input_fingerprint_sha256),
            input_fingerprint_json: Some(input_fingerprint_json),
            checkpoint_json: Some(checkpoint_json),
            artifacts_json,
            generation_run_ids_json: None,
            error_message: None,
            completed: true,
        })
        .await?;
    Ok(())
}
