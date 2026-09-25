use a4n_video::VideoGenerationResult;

use crate::AppState;
use crate::actions::video_generation::media::{
    best_effort_video_thumbnail_path, capture_video_tail_frame_asset, save_video_bytes,
};
use a4n_error::append_secondary_message;
use a4n_media_contract::MediaRoot;
use a4n_storage::{data_dir_relative_file_path, remove_created_file_on_error};
use a4n_video::{
    completed_provider_metadata, is_first_frame_chain_workflow, required_video_chapter_id,
    should_capture_video_tail_frame_fields,
};
use a4n_video_host_contract::{PersistedVideoResult, VideoProviderResultTarget};

pub(in crate::actions::video_generation) async fn persist_video_result(
    state: &AppState,
    target: &VideoProviderResultTarget,
    result: VideoGenerationResult,
) -> Result<PersistedVideoResult, String> {
    if result.bytes.is_empty() {
        return Err("视频 provider 返回了空文件，无法保存生成结果".to_string());
    }
    let file_size = i64::try_from(result.bytes.len())
        .map_err(|_| "视频文件过大，无法记录文件大小".to_string())?;
    let chapter_id = required_video_chapter_id(target.chapter_id, target.video_id)?;
    let path = save_video_bytes(
        state.data_dir(),
        target.project_id,
        chapter_id,
        target.video_id,
        &result.bytes,
        &result.format,
    )?;
    let persisted_path = path.clone();
    let video_path_str = remove_created_file_on_error(&path, "已写出的视频文件", async {
        data_dir_relative_file_path(state.data_dir(), &persisted_path, "视频文件")
    })
    .await?;
    let properties = remove_created_file_on_error(&path, "已写出的视频文件", async {
        state
            .probe_media_video_properties(&path, MediaRoot::Data, "生成视频")
            .await
            .map(|probe| probe.file_properties(file_size))
    })
    .await?;
    let thumbnail_path = best_effort_video_thumbnail_path(
        state,
        target.project_id,
        chapter_id,
        target.video_id,
        &path,
    )
    .await;
    let mut tail_frame_capture_error = None;
    let tail_frame_metadata = if should_capture_video_tail_frame_fields(
        &target.video_type,
        target.workflow_mode.as_deref(),
    ) {
        match capture_video_tail_frame_asset(state, target, chapter_id, &path).await {
            Ok(metadata) => Some(metadata),
            Err(msg) => {
                if is_first_frame_chain_workflow(target.workflow_mode.as_deref()) {
                    let message = format!("视频已生成但首帧串联尾帧采集失败：{msg}");
                    let cleanup = super::failure::cleanup_saved_video_outputs(
                        state,
                        target.video_id,
                        &path,
                        thumbnail_path.as_deref(),
                    )
                    .await;
                    return Err(append_secondary_message(message, cleanup));
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
    Ok(PersistedVideoResult {
        video_id: target.video_id,
        project_id: target.project_id,
        file_path: video_path_str,
        thumbnail_path,
        properties,
        metadata_json,
    })
}
