use super::*;

pub async fn poll_video_job(
    state: &AppState,
    user_id: i64,
    video_id: i64,
) -> Result<GeneratedVideoSummary, String> {
    let video = state
        .orchestrator()
        .generated_video_by_id(video_id)
        .await
        .map_err(crate::actions::app_error_message)?
        .ok_or_else(|| "视频记录不存在".to_string())?;
    access::verify_project_editor_string(state, user_id, video.project_id).await?;
    job_lifecycle::poll_video_job(state, video_id).await
}
