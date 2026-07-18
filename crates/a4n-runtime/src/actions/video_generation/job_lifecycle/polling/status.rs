use a4n_core::models::ai::generated_video;
use a4n_orchestrator::workflow::WORKFLOW_STATUS_RUNNING;

use super::super::workflow::mark_video_provider_workflow;
use crate::AppState;

pub(super) async fn mark_video_job_waiting(
    state: &AppState,
    video_id: i64,
    prompt_id: Option<i64>,
    status: &str,
) -> Result<generated_video::Model, String> {
    let updated = state
        .orchestrator
        .update_generated_video_waiting_status_and_prompt(video_id, prompt_id, status)
        .await
        .map_err(crate::actions::app_error_message)?;
    mark_video_provider_workflow(state, &updated, WORKFLOW_STATUS_RUNNING, true, None).await?;
    Ok(updated)
}
