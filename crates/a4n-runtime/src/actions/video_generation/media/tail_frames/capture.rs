use crate::AppState;
use crate::actions::video_generation::media::video_files::{
    capture_video_tail_frame, video_tail_frame_path,
};
use crate::cleanup_outbox::enqueue_file_artifact_cleanup_by_project;
use a4n_api_contract::video::VideoTailFrameInput;
use a4n_domain::image::IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE;
use a4n_storage::{data_dir_relative_file_path, data_file_record_facts};
use a4n_video_contract::VideoProviderResultTarget;
use serde_json::{Value, json};
use std::path::Path;

pub(crate) async fn capture_video_tail_frame_asset(
    state: &AppState,
    target: &VideoProviderResultTarget,
    chapter_id: i64,
    video_path: &Path,
) -> Result<Value, String> {
    let output = video_tail_frame_path(
        state.data_dir(),
        target.project_id,
        chapter_id,
        target.video_id,
    )?;
    let result = async {
        capture_video_tail_frame(video_path, &output)?;
        let (output_path_str, file_size) =
            data_file_record_facts(state.data_dir(), &output, "尾帧图片", "尾帧图片")?;
        let image = state
            .video_record_application()
            .create_video_tail_frame(VideoTailFrameInput {
                video_id: target.video_id,
                file_path: output_path_str,
                file_size,
            })
            .await
            .map_err(crate::actions::app_error_message)
            .map_err(|err| format!("尾帧图片记录写入失败：{err}"))?;
        Ok::<Value, String>(json!({
            "image_id": image.image_id,
            "file_path": image.file_path,
            "shot_number": image.shot_number,
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
                    &format!("tail-frame:{}:{path}", target.video_id),
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
