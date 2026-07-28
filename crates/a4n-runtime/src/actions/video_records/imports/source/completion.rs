use super::super::prelude::{
    AppState, Value, append_secondary_error, combine_optional_errors,
    delete_video_tail_frame_assets, generated_video,
};
use super::delete_video_record_error;
use crate::cleanup_outbox::enqueue_file_artifact_cleanup_by_project;

pub(in crate::actions::video_records) async fn mark_imported_video_completed(
    state: &AppState,
    record_id: i64,
    output_path_str: &str,
    thumbnail_path: Option<&str>,
    file_size: i64,
    prompt_id: Option<i64>,
    metadata_json: Option<Value>,
    failure_prefix: &str,
) -> Result<generated_video::Model, String> {
    match state
        .video()
        .mark_generated_video_completed_and_link_prompt(
            record_id,
            output_path_str,
            thumbnail_path,
            file_size,
            metadata_json,
            prompt_id,
        )
        .await
    {
        Ok(updated) => Ok(updated),
        Err(err) => {
            let mut paths = vec![output_path_str.to_string()];
            if let Some(path) = thumbnail_path
                .map(str::trim)
                .filter(|path| !path.is_empty())
            {
                paths.push(path.to_string());
            }
            let cleanup_error = enqueue_file_artifact_cleanup_by_project(
                state,
                &format!("video-completion-failure:{record_id}"),
                &paths,
            )
            .await
            .err();
            let cleanup_error = combine_optional_errors(
                cleanup_error,
                delete_video_tail_frame_assets(state, record_id).await.err(),
            );
            let delete_error = delete_video_record_error(&state.video(), record_id).await;
            Err(append_secondary_error(
                append_secondary_error(format!("{failure_prefix}：{err}"), cleanup_error),
                delete_error,
            ))
        }
    }
}
