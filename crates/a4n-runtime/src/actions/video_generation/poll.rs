use super::{GeneratedVideoSummary, job_lifecycle};
use crate::{AppState, actions::access};

pub async fn poll_video_job(
    state: &AppState,
    user_id: i64,
    video_id: i64,
) -> Result<GeneratedVideoSummary, String> {
    let video = state
        .orchestrator()
        .required_video_record(video_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    access::verify_project_editor_string(state, user_id, video.project_id).await?;
    job_lifecycle::poll_video_job(state, video).await
}
