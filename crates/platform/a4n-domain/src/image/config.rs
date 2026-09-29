use super::sizes::{ImageSize, validate_image_sizes};
use super::{
    CLIPROXY_IMAGE_BACKEND, COMFYUI_IMAGE_BACKEND, OPENAI_CHAT_IMAGE_BACKEND,
    OPENAI_COMPATIBLE_IMAGE_BACKEND, OPENAI_COMPATIBLE_IMAGE_EDIT_BACKEND,
    STABLE_DIFFUSION_IMAGE_BACKEND,
};
use crate::config::{
    normalize_config_name, normalize_optional_config_value, positive_i32, required_config_value,
};

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
    params.api_key_encrypted =
        normalize_image_backend_api_key(&params.backend, params.api_key_encrypted)?;
    params.base_url = crate::config::validate_http_base_url(
        &params.base_url,
        "图片生成配置",
        params.api_key_encrypted.as_deref(),
    )?;
    params.max_concurrent = positive_i32(params.max_concurrent, "图片配置 max_concurrent")?;
    validate_image_sizes(&params.supported_sizes)?;
    Ok(params)
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
