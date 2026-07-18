use super::super::prelude::{
    AppState, Path, Value, append_secondary_error, combine_optional_errors,
    delete_video_tail_frame_assets, generated_video, json, remove_partial_video_file,
};
use super::delete_video_record_error;

pub(in crate::actions::video_records) async fn mark_imported_video_completed(
    state: &AppState,
    record_id: i64,
    output: &Path,
    output_path_str: &str,
    thumbnail_path: Option<&str>,
    file_size: i64,
    prompt_id: Option<i64>,
    tail_frame_metadata: Option<&Value>,
    tail_frame_capture_error: Option<&str>,
    failure_prefix: &str,
    file_label: &str,
    thumbnail_label: &str,
) -> Result<generated_video::Model, String> {
    let completion_metadata = match (tail_frame_metadata, tail_frame_capture_error) {
        (Some(metadata), None) => Some(json!({ "tail_frame": metadata })),
        (Some(metadata), Some(error)) => {
            Some(json!({ "tail_frame": metadata, "tail_frame_capture_error": error }))
        }
        (None, Some(error)) => Some(json!({ "tail_frame_capture_error": error })),
        (None, None) => None,
    };
    match state
        .orchestrator
        .mark_generated_video_completed_and_link_prompt(
            record_id,
            output_path_str,
            thumbnail_path,
            file_size,
            completion_metadata,
            prompt_id,
        )
        .await
    {
        Ok(updated) => Ok(updated),
        Err(err) => {
            let cleanup_error = combine_optional_errors(
                remove_partial_video_file(output, file_label).err(),
                thumbnail_path.and_then(|path| {
                    remove_partial_video_file(&state.config.data_dir.join(path), thumbnail_label)
                        .err()
                }),
            );
            let cleanup_error = combine_optional_errors(
                cleanup_error,
                delete_video_tail_frame_assets(state, record_id).await.err(),
            );
            let delete_error = delete_video_record_error(state, record_id).await;
            Err(append_secondary_error(
                append_secondary_error(format!("{failure_prefix}：{err}"), cleanup_error),
                delete_error,
            ))
        }
    }
}
