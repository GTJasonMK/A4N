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
