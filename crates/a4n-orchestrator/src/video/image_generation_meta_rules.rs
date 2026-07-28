//! generated_image.metadata.generation_context 纯读取规则（无 DB）。

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoFrameImageGenerationContext<'a> {
    pub source: &'a str,
    pub prompt_id: i64,
    pub request_preview_note: Option<&'a str>,
}

pub fn video_frame_image_generation_context(
    metadata: Option<&Value>,
) -> Result<Option<VideoFrameImageGenerationContext<'_>>, String> {
    let Some(context) = metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(Value::as_object)
    else {
        return Ok(None);
    };
    let Some(source) = context
        .get("source")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    if !matches!(
        source,
        "video_boundary_frame_prompt" | "video_first_frame_prompt" | "video_storyboard_prompt"
    ) {
        return Ok(None);
    }
    let prompt_id = context
        .get("prompt_id")
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            format!(
                "{}请求缺少有效 generation_context.prompt_id，不能生成视频帧图片",
                video_frame_generation_label(source)
            )
        })?;
    let request_preview_note = context
        .get("request_preview_note")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    Ok(Some(VideoFrameImageGenerationContext {
        source,
        prompt_id,
        request_preview_note,
    }))
}

pub fn video_frame_generation_label(source: &str) -> &'static str {
    match source {
        "video_first_frame_prompt" => "首帧成图",
        "video_storyboard_prompt" => "分镜成图",
        _ => "边界帧成图",
    }
}

/// 从 generation_context 读取 prompt_id（正数）。
pub fn image_generation_prompt_id(metadata: Option<&Value>) -> Option<i64> {
    metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("prompt_id"))
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
}

/// 从 generation_context 读取 source 标记。
pub fn image_generation_source(metadata: Option<&Value>) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("source"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

/// 从 generation_context 读取 shot_number（正数）。
pub fn image_generation_shot_number(metadata: Option<&Value>) -> Option<i32> {
    let raw = metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("shot_number"))?;
    raw.as_i64()
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
}

/// 从 generation_context 读取 source_content_sha256。
pub fn image_generation_source_hash(metadata: Option<&Value>) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("source_content_sha256"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_and_rejects_invalid_generation_context() {
        let metadata = json!({
            "generation_context": {
                "prompt_id": 12,
                "source": " video_storyboard_prompt ",
                "shot_number": 3,
                "source_content_sha256": " abc "
            }
        });
        assert_eq!(image_generation_prompt_id(Some(&metadata)), Some(12));
        assert_eq!(
            image_generation_source(Some(&metadata)),
            Some("video_storyboard_prompt")
        );
        assert_eq!(image_generation_shot_number(Some(&metadata)), Some(3));
        assert_eq!(image_generation_source_hash(Some(&metadata)), Some("abc"));
        let invalid = json!({"generation_context": {"prompt_id": 0, "shot_number": -1}});
        assert_eq!(image_generation_prompt_id(Some(&invalid)), None);
        assert_eq!(image_generation_shot_number(Some(&invalid)), None);
    }

    #[test]
    fn parses_video_frame_generation_context() {
        let metadata = json!({
            "generation_context": {
                "source": "video_storyboard_prompt",
                "prompt_id": 44,
                "request_preview_note": "复用上一镜"
            }
        });
        let context = video_frame_image_generation_context(Some(&metadata))
            .expect("context should parse")
            .expect("video frame context should be present");

        assert_eq!(context.source, "video_storyboard_prompt");
        assert_eq!(context.prompt_id, 44);
        assert_eq!(context.request_preview_note, Some("复用上一镜"));
        assert_eq!(video_frame_generation_label(context.source), "分镜成图");

        let invalid = json!({
            "generation_context": {
                "source": "video_first_frame_prompt",
                "prompt_id": "43"
            }
        });
        assert!(
            video_frame_image_generation_context(Some(&invalid))
                .expect_err("prompt_id string must fail")
                .contains("generation_context.prompt_id")
        );
    }
}
