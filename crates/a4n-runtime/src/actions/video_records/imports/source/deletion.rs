use crate::state::VideoRuntimeFacade;

pub(in crate::actions::video_records) async fn delete_video_record_error(
    video: &VideoRuntimeFacade<'_>,
    record_id: i64,
) -> Option<String> {
    video
        .delete_generated_video_record(record_id)
        .await
        .err()
        .map(crate::actions::app_error_message)
}
