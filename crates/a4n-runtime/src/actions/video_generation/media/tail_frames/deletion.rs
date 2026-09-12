use crate::AppState;

pub(crate) async fn delete_video_tail_frame_assets(
    state: &AppState,
    video_id: i64,
) -> Result<(), String> {
    state
        .video_record_lifecycle_application()
        .delete_video_tail_frame_assets(video_id)
        .await
        .map_err(crate::actions::app_error_message)
}
