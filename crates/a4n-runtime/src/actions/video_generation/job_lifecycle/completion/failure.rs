use std::path::Path;

use a4n_core::models::ai::generated_video;
use a4n_orchestrator::workflow::WORKFLOW_STATUS_FAILED;

use super::super::workflow::mark_video_provider_workflow;
use crate::AppState;
use crate::actions::video_generation::fail_video_generation_runs;
use crate::actions::video_generation::media::{
    append_secondary_error, combine_optional_errors, delete_video_tail_frame_assets,
    remove_partial_video_file,
};

pub(super) async fn fail_saved_video_result(
    state: &AppState,
    video_id: i64,
    prompt_id: Option<i64>,
    video_path: &Path,
    thumbnail_path: Option<&str>,
    message: String,
) -> Result<generated_video::Model, String> {
    let cleanup_error =
        cleanup_saved_video_outputs(state, video_id, video_path, thumbnail_path).await;
    let failure_message = append_secondary_error(message.clone(), cleanup_error.clone());
    let updated = mark_video_failed_and_update_prompt(state, video_id, prompt_id, &failure_message)
        .await
        .map_err(|error| {
            append_secondary_error(
                failure_message,
                Some(format!("视频失败状态及关联运行收尾失败：{error}")),
            )
        })?;
    saved_video_failure_result(updated, message, cleanup_error)
}

pub(in crate::actions::video_generation::job_lifecycle) fn saved_video_failure_result<T>(
    value: T,
    message: String,
    cleanup_error: Option<String>,
) -> Result<T, String> {
    match cleanup_error {
        Some(cleanup_error) => Err(append_secondary_error(message, Some(cleanup_error))),
        None => Ok(value),
    }
}

async fn cleanup_saved_video_outputs(
    state: &AppState,
    video_id: i64,
    video_path: &Path,
    thumbnail_path: Option<&str>,
) -> Option<String> {
    combine_optional_errors(
        cleanup_saved_video_files(&state.config.data_dir, video_path, thumbnail_path),
        delete_video_tail_frame_assets(state, video_id).await.err(),
    )
}

pub(in crate::actions::video_generation::job_lifecycle) fn cleanup_saved_video_files(
    data_dir: &Path,
    video_path: &Path,
    thumbnail_path: Option<&str>,
) -> Option<String> {
    combine_optional_errors(
        remove_partial_video_file(video_path, "已生成视频文件").err(),
        thumbnail_path
            .map(|path| data_dir.join(path))
            .and_then(|path| remove_partial_video_file(&path, "视频缩略图").err()),
    )
}

pub(in crate::actions::video_generation::job_lifecycle) async fn mark_video_failed_and_update_prompt(
    state: &AppState,
    video_id: i64,
    prompt_id: Option<i64>,
    message: &str,
) -> Result<generated_video::Model, String> {
    let updated = state
        .orchestrator
        .mark_generated_video_error_status_and_prompt(video_id, prompt_id, "failed", message)
        .await
        .map_err(crate::actions::app_error_message)?;
    fail_video_generation_runs(state, video_id, message).await?;
    mark_video_provider_workflow(
        state,
        &updated,
        WORKFLOW_STATUS_FAILED,
        false,
        Some(message),
    )
    .await?;
    Ok(updated)
}
