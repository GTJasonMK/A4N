//! 视频尾帧归属与元数据纯规则（无 DB、无 Host）。

use a4n_core::models::ai::{generated_image, generated_video};
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_domain::image::{
    IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE, IMAGE_TYPE_VIDEO_TAIL_FRAME,
    image_generation_source,
};
use serde_json::Value;

/// 尾帧图是否属于指定视频的采集产物。
pub fn tail_frame_image_belongs_to_video(image: &generated_image::Model, video_id: i64) -> bool {
    tail_frame_metadata_belongs_to_video(&image.image_type, image.metadata_json.as_ref(), video_id)
}

/// 尾帧轻量引用是否属于指定视频，供只读清理列表避免加载完整图片实体。
pub fn tail_frame_reference_belongs_to_video(
    image: &generated_image::Reference,
    video_id: i64,
) -> bool {
    tail_frame_metadata_belongs_to_video(&image.image_type, image.metadata_json.as_ref(), video_id)
}

pub fn tail_frame_metadata_belongs_to_video(
    image_type: &str,
    metadata_json: Option<&Value>,
    video_id: i64,
) -> bool {
    if image_type != IMAGE_TYPE_VIDEO_TAIL_FRAME
        || image_generation_source(metadata_json)
            != Some(IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE)
    {
        return false;
    }
    metadata_json
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("video_id"))
        .and_then(Value::as_i64)
        == Some(video_id)
}

/// 从视频元数据读取镜头序号。
pub fn video_shot_number(video: &generated_video::Model) -> Result<i32, String> {
    video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("shot_number"))
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("视频 #{} 缺少有效 shot_number，无法采集真实尾帧", video.id))
}

pub fn is_first_frame_chain_video(video: &generated_video::Model) -> bool {
    video.workflow_mode.as_deref() == Some(chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN)
}

pub fn should_capture_video_tail_frame(video: &generated_video::Model) -> bool {
    video.video_type == generated_video::VIDEO_TYPE_CHAPTER
        && matches!(
            video.workflow_mode.as_deref(),
            Some(
                chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES
                    | chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN
                    | chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO
            )
        )
}

pub fn required_video_chapter_id(chapter_id: Option<i64>, video_id: i64) -> Result<i64, String> {
    chapter_id.ok_or_else(|| format!("视频 #{video_id} 缺少 chapter_id，无法保存生成结果"))
}

pub fn completed_provider_metadata(
    provider_metadata: Value,
    tail_frame_metadata: Option<Value>,
    tail_frame_capture_error: Option<String>,
) -> Option<Value> {
    if tail_frame_metadata.is_none() && tail_frame_capture_error.is_none() {
        return (!provider_metadata.is_null()).then_some(provider_metadata);
    }

    let mut metadata = serde_json::Map::new();
    if !provider_metadata.is_null() {
        metadata.insert("provider".to_string(), provider_metadata);
    }
    if let Some(tail_frame_metadata) = tail_frame_metadata {
        metadata.insert("tail_frame".to_string(), tail_frame_metadata);
    }
    if let Some(error) = tail_frame_capture_error {
        metadata.insert("tail_frame_capture_error".to_string(), Value::String(error));
    }
    Some(Value::Object(metadata))
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::image::IMAGE_TYPE_VIDEO_TAIL_FRAME;
    use chrono::Utc;
    use serde_json::json;

    fn image_with_context(context: Value) -> generated_image::Reference {
        generated_image::Reference {
            id: 1,
            image_type: IMAGE_TYPE_VIDEO_TAIL_FRAME.into(),
            metadata_json: Some(json!({ "generation_context": context })),
        }
    }

    fn video(video_type: &str, workflow_mode: Option<&str>) -> generated_video::Model {
        let now = Utc::now().naive_utc();
        generated_video::Model {
            id: 42,
            project_id: 1,
            chapter_id: Some(1),
            video_type: video_type.to_string(),
            workflow_mode: workflow_mode.map(str::to_string),
            file_path: None,
            thumbnail_path: None,
            prompt: "prompt".to_string(),
            negative_prompt: None,
            model: "model".to_string(),
            width: 1280,
            height: 720,
            duration_secs: 5.0,
            fps: 24,
            file_size: None,
            source_video_id: None,
            source_image_id: None,
            status: "completed".to_string(),
            provider_job_id: None,
            error_message: None,
            timeline_item_id: None,
            metadata_json: None,
            is_favorite: false,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn belongs_to_video_requires_capture_source_and_id() {
        let ok = image_with_context(json!({
            "source": IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE,
            "video_id": 42
        }));
        assert!(tail_frame_reference_belongs_to_video(&ok, 42));
        assert!(!tail_frame_reference_belongs_to_video(&ok, 41));

        let wrong_source = image_with_context(json!({
            "source": "other",
            "video_id": 42
        }));
        assert!(!tail_frame_reference_belongs_to_video(&wrong_source, 42));
    }

    #[test]
    fn provider_completion_rules_keep_chapter_and_tail_frame_contracts() {
        assert!(required_video_chapter_id(None, 42).is_err());
        for workflow in ["boundary_frames", "first_frame_chain", "chat_video"] {
            assert!(should_capture_video_tail_frame(&video(
                generated_video::VIDEO_TYPE_CHAPTER,
                Some(workflow),
            )));
        }
        assert!(!should_capture_video_tail_frame(&video(
            generated_video::VIDEO_TYPE_REFERENCE_MEDIA,
            Some("reference_media"),
        )));
        assert!(is_first_frame_chain_video(&video(
            generated_video::VIDEO_TYPE_CHAPTER,
            Some("first_frame_chain"),
        )));
    }

    #[test]
    fn completed_metadata_only_wraps_provider_when_tail_frame_fields_exist() {
        assert_eq!(completed_provider_metadata(Value::Null, None, None), None);
        assert_eq!(
            completed_provider_metadata(json!({"job_id": "job"}), None, None),
            Some(json!({"job_id": "job"}))
        );
        assert_eq!(
            completed_provider_metadata(
                json!({"job_id": "job"}),
                Some(json!({"image_id": 9})),
                Some("capture failed".to_string()),
            ),
            Some(json!({
                "provider": {"job_id": "job"},
                "tail_frame": {"image_id": 9},
                "tail_frame_capture_error": "capture failed",
            }))
        );
    }
}
