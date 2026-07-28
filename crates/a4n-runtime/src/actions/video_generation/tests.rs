use super::*;
use chrono::Utc;
use serde_json::Value;

fn reference_media_video_with_metadata(metadata_json: Option<Value>) -> generated_video::Model {
    let now = Utc::now().naive_utc();
    generated_video::Model {
        id: 42,
        project_id: 1,
        chapter_id: Some(2),
        video_type: "reference_media_video".to_string(),
        workflow_mode: Some("reference_media".to_string()),
        file_path: None,
        thumbnail_path: None,
        prompt: "这是一段足够长的参考媒体视频提示词，用于描述人物外观、场景结构、镜头调度、动作节奏、光影方向、情绪变化和画面风格；其中明确使用 @image_file_1 作为主角造型参考，并使用 @video_file_1 作为动作节奏参考。".to_string(),
        negative_prompt: None,
        model: "seedance-2-0-pro".to_string(),
        width: 1280,
        height: 720,
        duration_secs: 8.0,
        fps: 24,
        file_size: None,
        source_video_id: None,
        source_image_id: None,
        status: "completed".to_string(),
        provider_job_id: None,
        error_message: None,
        metadata_json,
        is_favorite: false,
        created_at: now,
        updated_at: now,
    }
}

mod reference_media;
mod results;
mod snapshot;
