use crate::AppState;
use crate::actions::video_generation::media::video_files::{
    capture_video_tail_frame, video_tail_frame_path,
};
use crate::cleanup_outbox::enqueue_file_artifact_cleanup_by_project;
use crate::file_utils::{data_dir_relative_file_path, data_file_record_facts};
use a4n_core::models::ai::generated_video;
use a4n_domain::image::IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE;
use a4n_orchestrator::video::video_shot_number;
use serde_json::{Value, json};
use std::path::Path;

pub(crate) async fn capture_video_tail_frame_asset(
    state: &AppState,
    video: &generated_video::Model,
    chapter_id: i64,
    video_path: &Path,
) -> Result<Value, String> {
    let shot_number = video_shot_number(video)?;
    let output = video_tail_frame_path(state.data_dir(), video.project_id, chapter_id, video.id)?;
    let result = async {
        capture_video_tail_frame(video_path, &output)?;
        let (output_path_str, file_size) =
            data_file_record_facts(state.data_dir(), &output, "尾帧图片", "尾帧图片")?;
        let image = state
            .orchestrator()
            .create_video_tail_frame_record_validated(
                video,
                chapter_id,
                output_path_str,
                file_size,
            )
            .await
            .map_err(crate::actions::app_error_message)
            .map_err(|err| format!("尾帧图片记录写入失败：{err}"))?;
        Ok::<Value, String>(json!({
            "image_id": image.id,
            "file_path": image.file_path,
            "shot_number": shot_number,
            "source": IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE,
        }))
    }
    .await;
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            let cleanup_error = if let Ok(path) =
                data_dir_relative_file_path(state.data_dir(), &output, "尾帧图片")
            {
                enqueue_file_artifact_cleanup_by_project(
                    state,
                    &format!("tail-frame:{}:{path}", video.id),
                    std::slice::from_ref(&path),
                )
                .await
                .err()
            } else {
                None
            };
            Err(
                crate::actions::video_generation::media::append_secondary_error(
                    error,
                    cleanup_error,
                ),
            )
        }
    }
}
