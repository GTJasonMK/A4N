use a4n_core::models::ai::generated_video;
use a4n_video::VideoGenerationResult;

use crate::AppState;
use crate::actions::video_generation::media::{
    append_secondary_error, best_effort_video_thumbnail_path, capture_video_tail_frame_asset,
    save_video_bytes,
};
use crate::file_utils::{data_dir_relative_file_path, remove_created_file_on_error};
use a4n_orchestrator::video::PersistedVideoResult;
use a4n_orchestrator::video::{
    completed_provider_metadata, is_first_frame_chain_video, required_video_chapter_id,
    should_capture_video_tail_frame,
};

pub(in crate::actions::video_generation) async fn persist_video_result(
    state: &AppState,
    video: &generated_video::Model,
    result: VideoGenerationResult,
) -> Result<PersistedVideoResult, String> {
    if result.bytes.is_empty() {
        return Err("视频 provider 返回了空文件，无法保存生成结果".to_string());
    }
    let chapter_id = required_video_chapter_id(video.chapter_id, video.id)?;
    let path = save_video_bytes(
        state.data_dir(),
        video.project_id,
        chapter_id,
        video.id,
        &result.bytes,
        &result.format,
    )?;
    let persisted_path = path.clone();
    let video_path_str = remove_created_file_on_error(&path, "已写出的视频文件", async {
        data_dir_relative_file_path(state.data_dir(), &persisted_path, "视频文件")
    })
    .await?;
    let thumbnail_path = best_effort_video_thumbnail_path(
        state.data_dir(),
        video.project_id,
        chapter_id,
        video.id,
        &path,
    );
    let mut tail_frame_capture_error = None;
    let tail_frame_metadata = if should_capture_video_tail_frame(video) {
        match capture_video_tail_frame_asset(state, video, chapter_id, &path).await {
            Ok(metadata) => Some(metadata),
            Err(msg) => {
                if is_first_frame_chain_video(video) {
                    let message = format!("视频已生成但首帧串联尾帧采集失败：{msg}");
                    let cleanup = super::failure::cleanup_saved_video_outputs(
                        state,
                        video.id,
                        &path,
                        thumbnail_path.as_deref(),
                    )
                    .await;
                    return Err(append_secondary_error(message, cleanup));
                }
                tail_frame_capture_error = Some(msg);
                None
            }
        }
    } else {
        None
    };
    let metadata_json = completed_provider_metadata(
        result.metadata,
        tail_frame_metadata,
        tail_frame_capture_error,
    );
    let file_size = i64::try_from(result.bytes.len())
        .map_err(|_| "视频文件过大，无法记录文件大小".to_string())?;
    Ok(PersistedVideoResult {
        video_id: video.id,
        project_id: video.project_id,
        file_path: video_path_str,
        thumbnail_path,
        file_size,
        metadata_json,
    })
}
