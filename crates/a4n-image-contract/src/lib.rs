//! Wire contracts for the image provider capability.
//!
//! This crate contains only serialized sidecar data. Provider implementations,
//! HTTP clients, and runtime state stay in their owning crates.

use serde::{Deserialize, Serialize};

pub const IMAGE_SERVICE_NAME: &str = "image";
pub const IMAGE_PROVIDER_CAPABILITY_VERSION: &str = "image-provider-v4";
pub const IMAGE_SERVICE_ADDR_ENV: &str = "A4N_IMAGE_SERVICE_ADDR";
pub const IMAGE_SERVICE_DEFAULT_ADDR: &str = "127.0.0.1:47645";

/// 单次生成请求。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImageGenerationRequest {
    /// 完整 prompt（已合成）。
    pub prompt: String,
    /// 输出宽度。
    pub width: i32,
    /// 输出高度。
    pub height: i32,
    /// 模型名（如 "dall-e-3"）。
    pub model: String,
    /// 参考图；不支持的 provider 应显式报错。
    pub reference_images: Vec<ImageReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImageReference {
    pub label: String,
    pub data_uri: String,
}

/// Image sidecar provider request payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImageProviderPayload {
    pub backend: String,
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    pub request: ImageGenerationRequest,
}

/// Image sidecar completed-generation payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImageGenerationOutput {
    pub bytes_base64: String,
    pub format: String,
    pub model: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_contract_rejects_unknown_fields() {
        let error = serde_json::from_value::<ImageProviderPayload>(serde_json::json!({
            "backend": "openai-compatible-image",
            "base_url": "https://example.invalid/v1",
            "request": {
                "prompt": "test",
                "width": 1024,
                "height": 1024,
                "model": "image-model",
                "reference_images": []
            },
            "legacy_mode": true
        }))
        .expect_err("unknown image payload fields must fail");
        assert!(error.to_string().contains("unknown field `legacy_mode`"));
    }
}
