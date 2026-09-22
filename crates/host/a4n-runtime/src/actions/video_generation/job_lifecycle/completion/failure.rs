use std::path::Path;

use crate::AppState;
use crate::actions::video_generation::media::delete_video_tail_frame_assets;
use crate::cleanup_outbox::enqueue_file_artifact_cleanup_by_project;
use a4n_error::combine_optional_errors;
use a4n_storage::data_dir_relative_file_path;

pub(crate) async fn cleanup_saved_video_outputs(
    state: &AppState,
    video_id: i64,
    video_path: &Path,
    thumbnail_path: Option<&str>,
) -> Option<String> {
    let cleanup_error =
        match data_dir_relative_file_path(state.data_dir(), video_path, "已生成视频文件") {
            Ok(video_path) => {
                let mut paths = vec![video_path.clone()];
                if let Some(path) = thumbnail_path
                    .map(str::trim)
                    .filter(|path| !path.is_empty())
                {
                    paths.push(path.to_string());
                }
                enqueue_file_artifact_cleanup_by_project(
                    state,
                    &format!("video-failure:{video_id}:{video_path}"),
                    &paths,
                )
                .await
                .err()
            }
            Err(error) => Some(error),
        };
    combine_optional_errors(
        cleanup_error,
        delete_video_tail_frame_assets(state, video_id).await.err(),
    )
}
