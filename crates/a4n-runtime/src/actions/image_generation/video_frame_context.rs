use serde_json::Value;

#[derive(Debug)]
pub(super) struct VideoFrameImageGenerationContext<'a> {
    pub(super) source: &'a str,
    pub(super) prompt_id: i64,
    pub(super) request_preview_note: Option<&'a str>,
}

pub(super) fn video_frame_image_generation_context(
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

pub(super) fn video_frame_generation_label(source: &str) -> &'static str {
    match source {
        "video_first_frame_prompt" => "首帧成图",
        "video_storyboard_prompt" => "分镜成图",
        _ => "边界帧成图",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_video_frame_generation_context_prompt_id_number() {
        let metadata = json!({
            "generation_context": {
                "source": "video_boundary_frame_prompt",
                "prompt_id": 42
            }
        });
        let context = video_frame_image_generation_context(Some(&metadata))
            .expect("context should parse")
            .expect("video frame context should be present");
        assert_eq!(context.source, "video_boundary_frame_prompt");
        assert_eq!(context.prompt_id, 42);
        assert_eq!(context.request_preview_note, None);
    }

    #[test]
    fn rejects_video_frame_generation_context_prompt_id_string() {
        let metadata = json!({
            "generation_context": {
                "source": "video_first_frame_prompt",
                "prompt_id": "43"
            }
        });
        let err = video_frame_image_generation_context(Some(&metadata))
            .expect_err("video frame generation must carry numeric prompt_id");

        assert!(err.contains("generation_context.prompt_id"));
    }

    #[test]
    fn parses_storyboard_frame_generation_context() {
        let metadata = json!({
            "generation_context": {
                "source": "video_storyboard_prompt",
                "prompt_id": 44
            }
        });
        let context = video_frame_image_generation_context(Some(&metadata))
            .expect("context should parse")
            .expect("storyboard frame context should be present");

        assert_eq!(context.source, "video_storyboard_prompt");
        assert_eq!(context.prompt_id, 44);
        assert_eq!(video_frame_generation_label(context.source), "分镜成图");
    }
}
