use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::config::{
    normalize_config_name, normalize_optional_config_value, positive_i32, required_config_value,
};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct CreateImageConfigParams {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub supported_sizes: Vec<ImageSize>,
}

pub const OPENAI_COMPATIBLE_IMAGE_BACKEND: &str = "openai-compatible-image";
pub const OPENAI_COMPATIBLE_IMAGE_EDIT_BACKEND: &str = "openai-compatible-image-edit";
pub const OPENAI_CHAT_IMAGE_BACKEND: &str = "openai-chat-image";
pub const CLIPROXY_IMAGE_BACKEND: &str = "cliproxy-image";
pub const STABLE_DIFFUSION_IMAGE_BACKEND: &str = "stable-diffusion";
pub const COMFYUI_IMAGE_BACKEND: &str = "comfyui";

/// 规范化立绘参考图 ID：正整数、去重保序。
pub fn validate_reference_image_ids(image_ids: &[i64]) -> Result<Vec<i64>, String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::with_capacity(image_ids.len());
    for image_id in image_ids.iter().copied() {
        if image_id <= 0 {
            return Err(format!("参考图 ID 必须为正整数：{image_id}"));
        }
        if !seen.insert(image_id) {
            return Err(format!("参考图包含重复 ID：{image_id}"));
        }
        normalized.push(image_id);
    }
    Ok(normalized)
}

/// 从 JSON 数组解析正整数参考图 ID，并拒绝重复。
pub fn reference_image_ids(value: Option<&Value>) -> Result<Vec<i64>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let values = value
        .as_array()
        .ok_or_else(|| "reference_image_ids 必须是数组".to_string())?;
    let mut ids = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let id = value
            .as_i64()
            .filter(|id| *id > 0)
            .ok_or_else(|| format!("reference_image_ids[{index}] 必须是正整数"))?;
        if ids.contains(&id) {
            return Err(format!("reference_image_ids 包含重复 ID：{id}"));
        }
        ids.push(id);
    }
    Ok(ids)
}

/// 校验已解析的参考图 ID 列表（正整数、无重复）。
pub fn validate_reference_image_id_list(ids: &[i64]) -> Result<(), String> {
    let mut seen = HashSet::with_capacity(ids.len());
    for (index, id) in ids.iter().copied().enumerate() {
        if id <= 0 {
            return Err(format!("reference_image_ids[{index}] 必须是正整数"));
        }
        if !seen.insert(id) {
            return Err(format!("reference_image_ids 包含重复 ID：{id}"));
        }
    }
    Ok(())
}

/// 从 batch 图片项上的 `reference_image_ids` 字段解析 ID 列表。
pub fn batch_item_reference_image_ids(item: &Value) -> Result<Vec<i64>, String> {
    let Some(value) = item
        .get("reference_image_ids")
        .filter(|value| !value.is_null())
    else {
        return Ok(Vec::new());
    };
    reference_image_ids(Some(value))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageProviderCapabilities {
    pub backend: &'static str,
    pub display_name: &'static str,
    pub requires_api_key: bool,
    pub supports_reference_images: bool,
}

pub const IMAGE_PROVIDER_CAPABILITIES: &[ImageProviderCapabilities] = &[
    ImageProviderCapabilities {
        backend: OPENAI_COMPATIBLE_IMAGE_BACKEND,
        display_name: "OpenAI 兼容图像",
        requires_api_key: true,
        supports_reference_images: false,
    },
    ImageProviderCapabilities {
        backend: OPENAI_COMPATIBLE_IMAGE_EDIT_BACKEND,
        display_name: "OpenAI 兼容图像编辑",
        requires_api_key: true,
        supports_reference_images: true,
    },
    ImageProviderCapabilities {
        backend: OPENAI_CHAT_IMAGE_BACKEND,
        display_name: "Chat Completions 图像",
        requires_api_key: true,
        supports_reference_images: true,
    },
    ImageProviderCapabilities {
        backend: CLIPROXY_IMAGE_BACKEND,
        display_name: "CLIProxy / Grok 图片",
        requires_api_key: true,
        supports_reference_images: true,
    },
    ImageProviderCapabilities {
        backend: STABLE_DIFFUSION_IMAGE_BACKEND,
        display_name: "Stable Diffusion WebUI",
        requires_api_key: false,
        supports_reference_images: false,
    },
    ImageProviderCapabilities {
        backend: COMFYUI_IMAGE_BACKEND,
        display_name: "ComfyUI",
        requires_api_key: false,
        supports_reference_images: false,
    },
];

pub fn image_provider_capabilities(backend: &str) -> Option<ImageProviderCapabilities> {
    IMAGE_PROVIDER_CAPABILITIES
        .iter()
        .copied()
        .find(|capabilities| capabilities.backend == backend)
}

pub fn image_backend_requires_api_key(backend: &str) -> bool {
    image_provider_capabilities(backend).is_some_and(|capabilities| capabilities.requires_api_key)
}

pub fn image_backend_supports_reference_images(backend: &str) -> bool {
    image_provider_capabilities(backend)
        .is_some_and(|capabilities| capabilities.supports_reference_images)
}

pub fn image_backend_api_key<'a>(
    backend: &str,
    api_key: Option<&'a str>,
) -> Result<&'a str, String> {
    let capabilities = image_provider_capabilities(backend)
        .ok_or_else(|| format!("未知图片生成后端: {backend}"))?;
    if capabilities.requires_api_key {
        return api_key
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "图片生成配置缺少 api_key_encrypted".into());
    }
    Ok("")
}

pub fn validate_image_config_params(
    mut params: CreateImageConfigParams,
) -> Result<CreateImageConfigParams, String> {
    params.name = normalize_config_name(params.name, "图片配置")?;
    params.backend = required_config_value(&params.backend, "图片生成配置", "backend")?;
    params.model = required_config_value(&params.model, "图片生成配置", "model")?;
    params.base_url = required_config_value(&params.base_url, "图片生成配置", "base_url")?;
    params.api_key_encrypted =
        normalize_image_backend_api_key(&params.backend, params.api_key_encrypted)?;
    params.max_concurrent = positive_i32(params.max_concurrent, "图片配置 max_concurrent")?;
    validate_image_sizes(&params.supported_sizes)?;
    Ok(params)
}

pub fn image_sizes_from_json(value: &Value) -> Result<Vec<ImageSize>, String> {
    let sizes = serde_json::from_value::<Vec<ImageSize>>(value.clone())
        .map_err(|error| format!("图片配置 supported_sizes 无效: {error}"))?;
    validate_image_sizes(&sizes)?;
    Ok(sizes)
}

pub fn image_sizes_to_json(sizes: &[ImageSize]) -> Result<Value, String> {
    validate_image_sizes(sizes)
        .map_err(|error| format!("图片配置 supported_sizes 无效: {error}"))?;
    serde_json::to_value(sizes).map_err(|error| format!("图片配置 supported_sizes 无效: {error}"))
}

pub fn ensure_image_size_supported(
    sizes: &[ImageSize],
    width: i32,
    height: i32,
) -> Result<(), String> {
    if sizes
        .iter()
        .any(|size| size.width == width && size.height == height)
    {
        return Ok(());
    }
    Err(format!(
        "图片尺寸 {}x{} 不在配置 supported_sizes 白名单中",
        width, height
    ))
}

fn normalize_image_backend_api_key(
    backend: &str,
    api_key_encrypted: Option<String>,
) -> Result<Option<String>, String> {
    let capabilities = image_provider_capabilities(backend)
        .ok_or_else(|| format!("未知图片生成后端: {backend}"))?;
    if capabilities.requires_api_key {
        return Ok(normalize_optional_config_value(api_key_encrypted));
    }
    Ok(None)
}

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
    fn reference_image_ids_are_positive_unique_and_ordered() {
        assert_eq!(validate_reference_image_ids(&[3, 5]).unwrap(), vec![3, 5]);
        assert_eq!(
            validate_reference_image_ids(&[]).unwrap(),
            Vec::<i64>::new()
        );

        let err = validate_reference_image_ids(&[0]).unwrap_err();
        assert!(err.contains("必须为正整数"));
        let err = validate_reference_image_ids(&[2, 2]).unwrap_err();
        assert!(err.contains("重复 ID：2"));
    }

    #[test]
    fn parses_reference_image_id_json_once_for_all_consumers() {
        assert_eq!(reference_image_ids(None), Ok(Vec::new()));
        assert_eq!(reference_image_ids(Some(&json!([3, 5]))), Ok(vec![3, 5]));
        assert_eq!(
            batch_item_reference_image_ids(&json!({"reference_image_ids": [1, 2]})),
            Ok(vec![1, 2])
        );
        assert!(reference_image_ids(Some(&json!([3, 3]))).is_err());
        assert!(validate_reference_image_id_list(&[0]).is_err());
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
