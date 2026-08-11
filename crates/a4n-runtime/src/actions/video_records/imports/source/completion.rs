use super::super::prelude::{
    AppState, Value, append_secondary_error, combine_optional_errors,
    delete_video_tail_frame_assets, generated_video,
};
use super::delete_video_record_error;
use crate::cleanup_outbox::enqueue_file_artifact_cleanup_by_project;
use crate::file_utils::canonicalize_data_file_path;

pub(in crate::actions::video_records) async fn enqueue_unadopted_video_file_cleanup(
    state: &AppState,
    dedupe_key: &str,
    output_path: &str,
    thumbnail_path: Option<&str>,
    authoritative: Option<&generated_video::Model>,
) -> Option<String> {
    let adopted = |path: &str| {
        authoritative.is_some_and(|video| {
            video.file_path.as_deref() == Some(path)
                || video.thumbnail_path.as_deref() == Some(path)
        })
    };
    let mut paths = Vec::with_capacity(2);
    if !adopted(output_path) {
        paths.push(output_path.to_string());
    }
    if let Some(path) = thumbnail_path
        .map(str::trim)
        .filter(|path| !path.is_empty() && !adopted(path) && *path != output_path)
    {
        paths.push(path.to_string());
    }
    if paths.is_empty() {
        return None;
    }
    enqueue_file_artifact_cleanup_by_project(state, dedupe_key, &paths)
        .await
        .err()
}

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
    let duration_secs = match canonicalize_data_file_path(
        state.data_dir(),
        output_path_str,
        &format!("{failure_prefix}输出文件"),
    )
    .and_then(|output_path| {
        super::probe_video_properties(&output_path, failure_prefix)
            .map(|properties| properties.duration_secs)
            .map_err(|error| format!("{failure_prefix}：{error}"))
    }) {
        Ok(duration_secs) => duration_secs,
        Err(error) => {
            let cleanup_error = enqueue_unadopted_video_file_cleanup(
                state,
                &format!("video-completion-probe-failure:{record_id}"),
                output_path_str,
                thumbnail_path,
                None,
            )
            .await;
            let cleanup_error = combine_optional_errors(
                cleanup_error,
                delete_video_tail_frame_assets(state, record_id).await.err(),
            );
            return Err(append_secondary_error(error, cleanup_error));
        }
    };
    match state
        .orchestrator()
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
        Ok(_) => state
            .orchestrator()
            .update_generated_video_duration_record(record_id, duration_secs)
            .await
            .map_err(|error| format!("{failure_prefix}实际时长回写失败：{error}")),
        Err(err) => {
            let can_adopt = a4n_orchestrator::video::can_adopt_authoritative_video_completion(&err);
            let primary_error = format!("{failure_prefix}：{err}");
            let authoritative = match state.orchestrator().generated_video_by_id(record_id).await {
                Ok(authoritative) => authoritative,
                Err(lookup_error) => {
                    return Err(append_secondary_error(
                        primary_error,
                        Some(format!(
                            "无法确认视频完成事务是否已提交，已保留文件等待重试：{}",
                            crate::actions::app_error_message(lookup_error)
                        )),
                    ));
                }
            };
            if let Some(authoritative) = authoritative.as_ref()
                && authoritative.status == a4n_orchestrator::video::GENERATED_VIDEO_STATUS_COMPLETED
            {
                let cleanup_error = enqueue_unadopted_video_file_cleanup(
                    state,
                    &format!("video-completion-duplicate:{record_id}"),
                    output_path_str,
                    thumbnail_path,
                    Some(authoritative),
                )
                .await;
                if can_adopt {
                    if let Some(cleanup_error) = cleanup_error {
                        tracing::error!(
                            video_id = record_id,
                            error = %cleanup_error,
                            "清理未采用的导入视频文件失败"
                        );
                    }
                    return Ok(authoritative.clone());
                }
                return Err(append_secondary_error(primary_error, cleanup_error));
            }
            let cleanup_error = enqueue_unadopted_video_file_cleanup(
                state,
                &format!("video-completion-failure:{record_id}"),
                output_path_str,
                thumbnail_path,
                None,
            )
            .await;
            let cleanup_error = combine_optional_errors(
                cleanup_error,
                delete_video_tail_frame_assets(state, record_id).await.err(),
            );
            if authoritative.as_ref().is_some_and(|video| {
                matches!(
                    video.status.as_str(),
                    a4n_orchestrator::video::GENERATED_VIDEO_ERROR_STATUS_FAILED
                        | a4n_orchestrator::video::GENERATED_VIDEO_ERROR_STATUS_INTERRUPTED
                        | a4n_orchestrator::video::GENERATED_VIDEO_STATUS_CANCELLED
                )
            }) {
                return Err(append_secondary_error(primary_error, cleanup_error));
            }
            let delete_error = delete_video_record_error(state.orchestrator(), record_id).await;
            Err(append_secondary_error(
                append_secondary_error(primary_error, cleanup_error),
                delete_error,
            ))
        }
    }
}
