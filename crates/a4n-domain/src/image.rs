use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImageSize {
    #[schemars(range(min = 1))]
    pub width: i32,
    #[schemars(range(min = 1))]
    pub height: i32,
}

impl ImageSize {
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
}

pub const DEFAULT_IMAGE_SIZES: &[ImageSize] = &[
    ImageSize::new(1024, 1024),
    ImageSize::new(1024, 1536),
    ImageSize::new(1536, 1024),
    ImageSize::new(1920, 1080),
    ImageSize::new(1080, 1920),
    ImageSize::new(1024, 768),
    ImageSize::new(768, 1024),
];

pub fn default_image_sizes() -> Vec<ImageSize> {
    DEFAULT_IMAGE_SIZES.to_vec()
}

pub fn validate_image_sizes(sizes: &[ImageSize]) -> Result<(), String> {
    if sizes.is_empty() {
        return Err("图片配置 supported_sizes 至少需要一个尺寸".to_string());
    }
    let mut seen = HashSet::with_capacity(sizes.len());
    for size in sizes {
        if size.width <= 0 || size.height <= 0 {
            return Err(format!(
                "图片配置 supported_sizes 包含无效尺寸 {}x{}",
                size.width, size.height
            ));
        }
        if !seen.insert((size.width, size.height)) {
            return Err(format!(
                "图片配置 supported_sizes 包含重复尺寸 {}x{}",
                size.width, size.height
            ));
        }
    }
    Ok(())
}

pub const IMAGE_TYPE_CANVAS: &str = "canvas_image";
pub const IMAGE_TYPE_CANVAS_FRAME: &str = "canvas_frame";
pub const IMAGE_TYPE_OBJECT_MATERIAL: &str = "object_material";
pub const IMAGE_TYPE_SCENE_MATERIAL: &str = "scene_material";
pub const IMAGE_TYPE_VIDEO_KEY_FRAME: &str = "video_key_frame";
pub const IMAGE_TYPE_VIDEO_START_FRAME: &str = "video_start_frame";
pub const IMAGE_TYPE_VIDEO_STORYBOARD_FRAME: &str = "video_storyboard_frame";
pub const IMAGE_TYPE_VIDEO_TAIL_FRAME: &str = "video_tail_frame";

pub const IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT: &str = "video_boundary_frame_prompt";
pub const IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT: &str = "video_first_frame_prompt";
pub const IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT: &str = "video_storyboard_prompt";
pub const IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE: &str = "video_tail_frame_capture";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoFrameImageGenerationContext<'a> {
    pub source: &'a str,
    pub generation_label: &'static str,
    pub prompt_id: i64,
}

pub fn video_frame_image_generation_context(
    metadata: Option<&Value>,
) -> Result<Option<VideoFrameImageGenerationContext<'_>>, String> {
    let Some(source) = image_generation_source(metadata) else {
        return Ok(None);
    };
    let Some(generation_label) = video_frame_prompt_generation_label(source) else {
        return Ok(None);
    };
    let prompt_id = image_generation_prompt_id(metadata).ok_or_else(|| {
        format!("{generation_label}请求缺少有效 generation_context.prompt_id，不能生成视频帧图片")
    })?;
    Ok(Some(VideoFrameImageGenerationContext {
        source,
        generation_label,
        prompt_id,
    }))
}

pub fn is_video_frame_prompt_image_type(image_type: &str) -> bool {
    matches!(
        image_type,
        IMAGE_TYPE_VIDEO_KEY_FRAME
            | IMAGE_TYPE_VIDEO_START_FRAME
            | IMAGE_TYPE_VIDEO_STORYBOARD_FRAME
    )
}

fn video_frame_prompt_generation_label(source: &str) -> Option<&'static str> {
    match source {
        IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT => Some("边界帧成图"),
        IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT => Some("首帧成图"),
        IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT => Some("分镜成图"),
        _ => None,
    }
}

pub fn image_generation_prompt_id(metadata: Option<&Value>) -> Option<i64> {
    image_generation_context(metadata)
        .and_then(|context| context.get("prompt_id"))
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
}

pub fn image_generation_source(metadata: Option<&Value>) -> Option<&str> {
    image_generation_context(metadata)
        .and_then(|context| context.get("source"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && *value == value.trim())
}

pub fn image_generation_shot_number(metadata: Option<&Value>) -> Option<i32> {
    image_generation_context(metadata)
        .and_then(|context| context.get("shot_number"))
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
}

pub fn image_generation_source_hash(metadata: Option<&Value>) -> Option<&str> {
    image_generation_context(metadata)
        .and_then(|context| context.get("source_content_sha256"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && *value == value.trim())
}

fn image_generation_context(metadata: Option<&Value>) -> Option<&serde_json::Map<String, Value>> {
    metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(Value::as_object)
}

pub fn is_system_managed_image_type(image_type: &str) -> bool {
    matches!(
        image_type.trim(),
        "scene"
            | "manga_page"
            | IMAGE_TYPE_SCENE_MATERIAL
            | IMAGE_TYPE_OBJECT_MATERIAL
            | IMAGE_TYPE_VIDEO_KEY_FRAME
            | IMAGE_TYPE_VIDEO_START_FRAME
            | IMAGE_TYPE_VIDEO_STORYBOARD_FRAME
            | IMAGE_TYPE_VIDEO_TAIL_FRAME
            | IMAGE_TYPE_CANVAS
            | IMAGE_TYPE_CANVAS_FRAME
            | "portrait"
            | "portrait_reference"
            | "chapter_reference"
            | "project_reference"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn system_managed_image_types_have_one_canonical_catalog() {
        assert!(is_system_managed_image_type(" project_reference "));
        assert!(is_system_managed_image_type(IMAGE_TYPE_VIDEO_TAIL_FRAME));
        assert!(!is_system_managed_image_type("custom_reference"));
    }

    #[test]
    fn image_generation_metadata_has_one_strict_reader() {
        let metadata = json!({
            "generation_context": {
                "prompt_id": 12,
                "source": IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT,
                "shot_number": 3,
                "source_content_sha256": "abc"
            }
        });
        assert_eq!(image_generation_prompt_id(Some(&metadata)), Some(12));
        assert_eq!(
            image_generation_source(Some(&metadata)),
            Some(IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT)
        );
        assert_eq!(image_generation_shot_number(Some(&metadata)), Some(3));
        assert_eq!(image_generation_source_hash(Some(&metadata)), Some("abc"));

        let invalid = json!({
            "generation_context": {
                "prompt_id": 0,
                "source": " video_storyboard_prompt ",
                "shot_number": -1,
                "source_content_sha256": " abc "
            }
        });
        assert_eq!(image_generation_prompt_id(Some(&invalid)), None);
        assert_eq!(image_generation_source(Some(&invalid)), None);
        assert_eq!(image_generation_shot_number(Some(&invalid)), None);
        assert_eq!(image_generation_source_hash(Some(&invalid)), None);
    }

    #[test]
    fn video_frame_generation_context_rejects_unknown_and_invalid_sources() {
        let metadata = json!({
            "generation_context": {
                "source": IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT,
                "prompt_id": 44
            }
        });
        let context = video_frame_image_generation_context(Some(&metadata))
            .expect("context should parse")
            .expect("video frame context should be present");
        assert_eq!(
            context.source,
            IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT
        );
        assert_eq!(context.generation_label, "分镜成图");
        assert_eq!(context.prompt_id, 44);

        let invalid = json!({
            "generation_context": {
                "source": IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT,
                "prompt_id": "43"
            }
        });
        assert!(
            video_frame_image_generation_context(Some(&invalid))
                .expect_err("prompt_id string must fail")
                .contains("generation_context.prompt_id")
        );
        assert_eq!(
            video_frame_image_generation_context(Some(&json!({
                "generation_context": {
                    "source": IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE
                }
            })))
            .expect("tail capture is not a prompt image source"),
            None
        );
    }
}
