//! Pure video completion rules shared by Host and the video owner.

use serde_json::Value;

pub fn tail_frame_metadata_belongs_to_video(
    image_type: &str,
    metadata_json: Option<&Value>,
    video_id: i64,
) -> bool {
    if image_type != a4n_domain::image::IMAGE_TYPE_VIDEO_TAIL_FRAME
        || a4n_domain::image::image_generation_source(metadata_json)
            != Some(a4n_domain::image::IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE)
    {
        return false;
    }
    metadata_json
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("video_id"))
        .and_then(Value::as_i64)
        == Some(video_id)
}

pub fn video_shot_number(metadata_json: Option<&Value>, video_id: i64) -> Result<i32, String> {
    metadata_json
        .and_then(|metadata| metadata.get("shot_number"))
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("视频 #{video_id} 缺少有效 shot_number，无法采集真实尾帧"))
}

pub fn is_first_frame_chain_workflow(workflow_mode: Option<&str>) -> bool {
    workflow_mode == Some(a4n_domain::video_workflow::WORKFLOW_MODE_FIRST_FRAME_CHAIN)
}

pub fn should_capture_video_tail_frame_fields(
    video_type: &str,
    workflow_mode: Option<&str>,
) -> bool {
    video_type == a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER
        && matches!(
            workflow_mode,
            Some(
                a4n_domain::video_workflow::WORKFLOW_MODE_BOUNDARY_FRAMES
                    | a4n_domain::video_workflow::WORKFLOW_MODE_FIRST_FRAME_CHAIN
                    | a4n_domain::video_workflow::WORKFLOW_MODE_CHAT_VIDEO
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
    use serde_json::json;

    #[test]
    fn tail_frame_metadata_requires_capture_source_and_video_id() {
        let metadata = json!({
            "generation_context": {
                "source": a4n_domain::image::IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE,
                "video_id": 42,
            }
        });
        assert!(tail_frame_metadata_belongs_to_video(
            a4n_domain::image::IMAGE_TYPE_VIDEO_TAIL_FRAME,
            Some(&metadata),
            42,
        ));
        assert!(!tail_frame_metadata_belongs_to_video(
            a4n_domain::image::IMAGE_TYPE_VIDEO_TAIL_FRAME,
            Some(&metadata),
            41,
        ));
        assert!(!tail_frame_metadata_belongs_to_video(
            "illustration",
            Some(&metadata),
            42,
        ));
    }

    #[test]
    fn video_shot_number_requires_positive_integer_metadata() {
        assert_eq!(
            video_shot_number(Some(&json!({"shot_number": 3})), 42),
            Ok(3)
        );
        assert!(video_shot_number(Some(&json!({"shot_number": 0})), 42).is_err());
        assert!(video_shot_number(Some(&json!({"shot_number": "3"})), 42).is_err());
    }

    #[test]
    fn provider_completion_rules_keep_chapter_and_tail_frame_contracts() {
        assert!(required_video_chapter_id(None, 42).is_err());
        for workflow in [
            a4n_domain::video_workflow::WORKFLOW_MODE_BOUNDARY_FRAMES,
            a4n_domain::video_workflow::WORKFLOW_MODE_FIRST_FRAME_CHAIN,
            a4n_domain::video_workflow::WORKFLOW_MODE_CHAT_VIDEO,
        ] {
            assert!(should_capture_video_tail_frame_fields(
                a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER,
                Some(workflow),
            ));
        }
        assert!(!should_capture_video_tail_frame_fields(
            a4n_domain::video_workflow::VIDEO_TYPE_REFERENCE_MEDIA,
            Some(a4n_domain::video_workflow::WORKFLOW_MODE_REFERENCE_MEDIA),
        ));
        assert!(is_first_frame_chain_workflow(Some(
            a4n_domain::video_workflow::WORKFLOW_MODE_FIRST_FRAME_CHAIN,
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
