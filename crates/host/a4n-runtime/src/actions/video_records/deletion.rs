use crate::AppState;
use crate::actions::app_error_message;
use a4n_api_contract::video::VideoRecordLifecycleApplicationPort;
use a4n_error::append_secondary_message;

pub(in crate::actions::video_records) async fn delete_video_record_cleanup_error(
    video: &dyn VideoRecordLifecycleApplicationPort,
    record_id: i64,
) -> Option<String> {
    video
        .delete_record_with_cleanup(record_id)
        .await
        .err()
        .map(crate::actions::app_error_message)
}

pub(in crate::actions::video_records) async fn append_video_record_cleanup_error<T>(
    video: &dyn VideoRecordLifecycleApplicationPort,
    record_id: i64,
    result: Result<T, String>,
) -> Result<T, String> {
    match result {
        Ok(result) => Ok(result),
        Err(error) => Err(append_secondary_message(
            error,
            delete_video_record_cleanup_error(video, record_id).await,
        )),
    }
}

pub async fn delete_video(state: &AppState, user_id: i64, video_id: i64) -> Result<(), String> {
    state
        .video_record_mutation_application()
        .delete(user_id, video_id)
        .await
        .map_err(app_error_message)
}
