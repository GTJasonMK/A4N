use crate::AppState;
use crate::actions::access;
use crate::actions::app_error_message;
use crate::actions::video_generation::media::delete_video_tail_frame_assets;
use a4n_core::error::AppError;
use a4n_orchestrator::video::VideoRecordLifecycleHostPort;
use async_trait::async_trait;

struct VideoRecordLifecycleHost<'a> {
    state: &'a AppState,
}

#[async_trait]
impl VideoRecordLifecycleHostPort for VideoRecordLifecycleHost<'_> {
    async fn delete_video_tail_frame_assets(&self, video_id: i64) -> Result<(), AppError> {
        delete_video_tail_frame_assets(self.state, video_id)
            .await
            .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))
    }
}

pub async fn delete_video(state: &AppState, user_id: i64, video_id: i64) -> Result<(), String> {
    let project_ids = state
        .video()
        .missing_video_reference_project_ids(video_id)
        .await
        .map_err(app_error_message)?;
    if project_ids.is_empty() {
        return Err("视频不存在".to_string());
    }
    for project_id in project_ids {
        access::verify_project_editor_string(state, user_id, project_id).await?;
    }
    state
        .video()
        .delete_video_record_validated(&VideoRecordLifecycleHost { state }, video_id)
        .await
        .map_err(app_error_message)
}
