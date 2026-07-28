//! 视频尾帧归属与元数据纯规则（无 DB、无 Host）。

use a4n_core::models::ai::{generated_image, generated_video};
use serde_json::Value;

/// 尾帧图是否属于指定视频的采集产物。
pub fn tail_frame_image_belongs_to_video(image: &generated_image::Model, video_id: i64) -> bool {
    let Some(context) = image
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("generation_context"))
    else {
        return false;
    };
    context
        .get("source")
        .and_then(Value::as_str)
        .is_some_and(|source| source.trim() == "video_tail_frame_capture")
        && context.get("video_id").and_then(Value::as_i64) == Some(video_id)
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

/// 从视频元数据读取源正文哈希。
pub fn video_source_content_hash(metadata: Option<&Value>) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("source"))
        .and_then(|source| source.get("content_sha256"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

pub fn is_first_frame_chain_video(video: &generated_video::Model) -> bool {
    video.workflow_mode.as_deref() == Some("first_frame_chain")
}

pub fn should_capture_video_tail_frame(video: &generated_video::Model) -> bool {
    video.video_type == "chapter_video"
        && matches!(
            video.workflow_mode.as_deref(),
            Some("boundary_frames" | "first_frame_chain" | "chat_video")
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
    use chrono::Utc;
    use serde_json::json;

    fn image_with_context(context: Value) -> generated_image::Model {
        let ts = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        generated_image::Model {
            id: 1,
            project_id: 1,
            image_type: "video_tail_frame".into(),
            file_path: "x.png".into(),
            prompt: String::new(),
            model: String::new(),
            width: 0,
            height: 0,
            file_size: None,
            ai_score: None,
            is_favorite: false,
            chapter_id: None,
            panel_id: None,
            metadata_json: Some(json!({ "generation_context": context })),
            created_at: ts,
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
            metadata_json: None,
            is_favorite: false,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn belongs_to_video_requires_capture_source_and_id() {
        let ok = image_with_context(json!({
            "source": "video_tail_frame_capture",
            "video_id": 42
        }));
        assert!(tail_frame_image_belongs_to_video(&ok, 42));
        assert!(!tail_frame_image_belongs_to_video(&ok, 41));

        let wrong_source = image_with_context(json!({
            "source": "other",
            "video_id": 42
        }));
        assert!(!tail_frame_image_belongs_to_video(&wrong_source, 42));
    }

    #[test]
    fn source_content_hash_reads_nested_source() {
        let meta = json!({"source": {"content_sha256": "abc"}});
        assert_eq!(video_source_content_hash(Some(&meta)), Some("abc"));
        assert_eq!(video_source_content_hash(None), None);
    }

    #[test]
    fn provider_completion_rules_keep_chapter_and_tail_frame_contracts() {
        assert!(required_video_chapter_id(None, 42).is_err());
        for workflow in ["boundary_frames", "first_frame_chain", "chat_video"] {
            assert!(should_capture_video_tail_frame(&video(
                "chapter_video",
                Some(workflow),
            )));
        }
        assert!(!should_capture_video_tail_frame(&video(
            "reference_media_video",
            Some("reference_media"),
        )));
        assert!(is_first_frame_chain_video(&video(
            "chapter_video",
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
