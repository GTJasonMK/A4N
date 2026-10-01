use super::*;
use a4n_video::VideoGenerationResult;
use chrono::Utc;
use serde_json::{Value, json};

fn reference_media_video_with_metadata(metadata_json: Option<Value>) -> GeneratedVideoSummary {
    let now = Utc::now().naive_utc();
    GeneratedVideoSummary {
        id: 42,
        project_id: 1,
        chapter_id: Some(2),
        video_type: a4n_domain::video_workflow::VIDEO_TYPE_REFERENCE_MEDIA.to_string(),
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
        timeline_item_id: None,
        metadata_json,
        is_favorite: false,
        created_at: now.to_string(),
        updated_at: now.to_string(),
    }
}

#[test]
fn completed_video_metadata_discards_provider_media_payload() {
    let result = VideoGenerationResult {
        bytes: b"video-bytes".to_vec(),
        format: "mp4".to_string(),
        provider_job_id: Some("provider-job".to_string()),
        metadata: json!({
            "data": [{"b64_json": "large-media-base64", "format": "mp4"}],
        }),
    };

    let metadata = super::preview::completed_video_artifact_metadata(&result);

    assert_eq!(metadata["provider_job_id"], json!("provider-job"));
    assert_eq!(metadata["artifact"]["format"], json!("mp4"));
    assert_eq!(metadata["artifact"]["byte_size"], json!(11));
    assert_eq!(
        metadata["artifact"]["sha256"],
        json!("79fd615a866fe7f9eb4da8d9c41ab57e3bd48056df42fd2c13e4d461a87afbe3")
    );
    assert!(!metadata.to_string().contains("b64_json"));
    assert!(!metadata.to_string().contains("large-media-base64"));
}

mod results;
mod snapshot;
