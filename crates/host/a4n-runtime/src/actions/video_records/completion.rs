use crate::AppState;
use crate::actions::app_error_message;
use crate::actions::video_generation::media::delete_video_tail_frame_assets;
use crate::actions::video_records::{delete_video_record_cleanup_error, probe_video_properties};
use crate::cleanup_outbox::enqueue_file_artifact_cleanup_by_project;
use a4n_api_contract::video::{
    CompleteVideoRecordInput, GeneratedVideoSummary, VideoRecordCompletionError,
};
use a4n_domain::media::{MEDIA_STATUS_CANCELLED, MEDIA_STATUS_COMPLETED, MEDIA_STATUS_FAILED};
use a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED;
use a4n_error::{append_secondary_message, combine_optional_errors};
use a4n_media_contract::MediaRoot;
use a4n_storage::canonicalize_data_file_path;
use serde_json::Value;

pub(in crate::actions::video_records) trait VideoRecordCleanupFacts:
    Send + Sync
{
    fn file_path(&self) -> Option<&str>;
    fn thumbnail_path(&self) -> Option<&str>;
}

impl VideoRecordCleanupFacts for GeneratedVideoSummary {
    fn file_path(&self) -> Option<&str> {
        self.file_path.as_deref()
    }

    fn thumbnail_path(&self) -> Option<&str> {
        self.thumbnail_path.as_deref()
    }
}

pub(in crate::actions::video_records) async fn enqueue_unadopted_video_file_cleanup(
    state: &AppState,
    dedupe_key: &str,
    output_path: &str,
    thumbnail_path: Option<&str>,
    authoritative: Option<&dyn VideoRecordCleanupFacts>,
) -> Option<String> {
    let adopted = |path: &str| {
        authoritative.is_some_and(|video| {
            video.file_path() == Some(path) || video.thumbnail_path() == Some(path)
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

pub(in crate::actions::video_records) async fn complete_video_record(
    state: &AppState,
    record_id: i64,
    output_path_str: &str,
    thumbnail_path: Option<&str>,
    file_size: i64,
    prompt_id: Option<i64>,
    metadata_json: Option<Value>,
    failure_prefix: &str,
) -> Result<GeneratedVideoSummary, String> {
    let duration_secs = match canonicalize_data_file_path(
        state.data_dir(),
        output_path_str,
        &format!("{failure_prefix}输出文件"),
    ) {
        Err(error) => Err(error),
        Ok(output_path) => {
            probe_video_properties(state, &output_path, MediaRoot::Data, failure_prefix)
                .await
                .map(|properties| properties.duration_secs)
                .map_err(|error| format!("{failure_prefix}：{error}"))
        }
    };
    let duration_secs = match duration_secs {
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
            return Err(append_secondary_message(error, cleanup_error));
        }
    };
    match state
        .video_record_completion_application()
        .complete(CompleteVideoRecordInput {
            record_id,
            output_path: output_path_str.to_string(),
            thumbnail_path: thumbnail_path.map(str::to_string),
            file_size,
            prompt_id,
            metadata_json,
            duration_secs,
        })
        .await
    {
        Ok(completion) => {
            if completion.adopted_authoritative
                && let Some(cleanup_error) = enqueue_unadopted_video_file_cleanup(
                    state,
                    &format!("video-completion-duplicate:{record_id}"),
                    output_path_str,
                    thumbnail_path,
                    Some(&completion.summary),
                )
                .await
            {
                tracing::error!(
                    video_id = record_id,
                    error = %cleanup_error,
                    "清理未采用的视频文件失败"
                );
            }
            Ok(completion.summary)
        }
        Err(VideoRecordCompletionError::DurationWriteback { error }) => {
            Err(format!("{failure_prefix}实际时长回写失败：{error}"))
        }
        Err(VideoRecordCompletionError::Submission {
            error,
            authoritative,
            lookup_error,
        }) => {
            let primary_error = format!("{failure_prefix}：{error}");
            if let Some(lookup_error) = lookup_error {
                return Err(append_secondary_message(
                    primary_error,
                    Some(format!(
                        "无法确认视频完成事务是否已提交，已保留文件等待重试：{}",
                        app_error_message(lookup_error)
                    )),
                ));
            }
            if let Some(authoritative) = authoritative.as_ref()
                && authoritative.summary.status == MEDIA_STATUS_COMPLETED
            {
                let cleanup_error = enqueue_unadopted_video_file_cleanup(
                    state,
                    &format!("video-completion-duplicate:{record_id}"),
                    output_path_str,
                    thumbnail_path,
                    Some(&authoritative.summary),
                )
                .await;
                return Err(append_secondary_message(primary_error, cleanup_error));
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
            if authoritative.as_ref().is_some_and(|completion| {
                matches!(
                    completion.summary.status.as_str(),
                    MEDIA_STATUS_FAILED | VIDEO_STATUS_INTERRUPTED | MEDIA_STATUS_CANCELLED
                )
            }) {
                return Err(append_secondary_message(primary_error, cleanup_error));
            }
            let delete_error = delete_video_record_cleanup_error(
                state.video_record_lifecycle_application(),
                record_id,
            )
            .await;
            Err(append_secondary_message(
                append_secondary_message(primary_error, cleanup_error),
                delete_error,
            ))
        }
    }
}
