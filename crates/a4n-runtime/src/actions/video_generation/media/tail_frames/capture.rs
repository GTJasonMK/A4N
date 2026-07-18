use super::rules::{video_shot_number, video_source_content_hash};
use crate::AppState;
use crate::actions::video_generation::media::video_files::{
    capture_video_tail_frame, video_tail_frame_path,
};
use crate::file_utils::{data_dir_relative_file_path, remove_created_file_on_error};
use a4n_core::models::ai::generated_video;
use a4n_orchestrator::video::CreateGeneratedImageRecordDraftInput;
use serde_json::{Value, json};
use std::path::Path;

pub(crate) async fn capture_video_tail_frame_asset(
    state: &AppState,
    video: &generated_video::Model,
    chapter_id: i64,
    prompt_id: Option<i64>,
    video_path: &Path,
) -> Result<Value, String> {
    let shot_number = video_shot_number(video)?;
    let source = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("source"))
        .cloned();
    let metadata = json!({
        "generation_context": {
            "source": "video_tail_frame_capture",
            "surface": "novel_detail.video",
            "frame_kind": "actual_tail_frame",
            "project_id": video.project_id,
            "chapter_id": chapter_id,
            "video_id": video.id,
            "prompt_id": prompt_id,
            "shot_number": shot_number,
            "source_content_sha256": video_source_content_hash(video.metadata_json.as_ref()),
        },
        "source": source,
    });
    let output = video_tail_frame_path(state, video.project_id, chapter_id, video.id)?;
    let persisted_output = output.clone();
    remove_created_file_on_error(&output, "已写出的尾帧图片", async move {
        capture_video_tail_frame(video_path, &persisted_output)?;
        let output_path_str =
            data_dir_relative_file_path(&state.config.data_dir, &persisted_output, "尾帧图片")?;
        let file_size = std::fs::metadata(&persisted_output)
            .map_err(|err| format!("无法读取尾帧图片元数据：{err}"))?
            .len() as i64;
        let image = state
            .orchestrator
            .create_generated_image_record(CreateGeneratedImageRecordDraftInput {
                project_id: video.project_id,
                image_type: "video_tail_frame".to_string(),
                file_path: output_path_str,
                prompt: format!("视频 #{} 第 {} 段真实尾帧采集", video.id, shot_number),
                model: "ffmpeg".to_string(),
                width: video.width.max(1),
                height: video.height.max(1),
                file_size: Some(file_size),
                chapter_id: Some(chapter_id),
                panel_id: None,
                metadata_json: Some(metadata),
                created_at: None,
            })
            .await
            .map_err(crate::actions::app_error_message)
            .map_err(|err| format!("尾帧图片记录写入失败：{err}"))?;
        Ok(json!({
            "image_id": image.id,
            "file_path": image.file_path,
            "shot_number": shot_number,
            "source": "video_tail_frame_capture",
        }))
    })
    .await
}
