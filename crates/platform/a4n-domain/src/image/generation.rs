use serde_json::Value;

pub const IMAGE_TYPE_CANVAS: &str = "canvas_image";
pub const IMAGE_TYPE_CANVAS_FRAME: &str = "canvas_frame";
pub const IMAGE_TYPE_OBJECT_MATERIAL: &str = "object_material";
pub const IMAGE_TYPE_SCENE_MATERIAL: &str = "scene_material";
pub const IMAGE_TYPE_VIDEO_KEY_FRAME: &str = "video_key_frame";
pub const IMAGE_TYPE_VIDEO_START_FRAME: &str = "video_start_frame";
pub const IMAGE_TYPE_VIDEO_STORYBOARD_FRAME: &str = "video_storyboard_frame";
pub const IMAGE_TYPE_DIRECTOR_STORYBOARD_FRAME: &str = "director_storyboard_frame";
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
            | IMAGE_TYPE_DIRECTOR_STORYBOARD_FRAME
            | IMAGE_TYPE_CANVAS
            | IMAGE_TYPE_CANVAS_FRAME
            | "portrait"
            | "portrait_reference"
            | "chapter_reference"
            | "project_reference"
    )
}
