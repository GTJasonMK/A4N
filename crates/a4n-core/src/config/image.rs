use crate::{error::AppError, models::ai::image_gen_config};

use super::{
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
    pub default_width: i32,
    pub default_height: i32,
}

#[derive(Debug, Clone)]
pub struct UpdateImageConfigParams {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    pub api_key_encrypted: Option<Option<String>>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
}

pub const OPENAI_COMPATIBLE_IMAGE_BACKEND: &str = "openai-compatible-image";
pub const OPENAI_COMPATIBLE_IMAGE_EDIT_BACKEND: &str = "openai-compatible-image-edit";
pub const OPENAI_CHAT_IMAGE_BACKEND: &str = "openai-chat-image";
pub const CLIPROXY_IMAGE_BACKEND: &str = "cliproxy-image";
pub const STABLE_DIFFUSION_IMAGE_BACKEND: &str = "stable-diffusion";
pub const COMFYUI_IMAGE_BACKEND: &str = "comfyui";

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
) -> Result<&'a str, AppError> {
    let capabilities = image_provider_capabilities(backend)
        .ok_or_else(|| AppError::InvalidArgument(format!("未知图片生成后端: {backend}")))?;
    if capabilities.requires_api_key {
        return api_key
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::InvalidArgument("图片生成配置缺少 api_key_encrypted".into()));
    }
    Ok("")
}

pub fn validate_image_config_params(
    mut params: CreateImageConfigParams,
) -> Result<CreateImageConfigParams, AppError> {
    params.name = normalize_config_name(params.name, "图片配置")?;
    params.backend = required_config_value(&params.backend, "图片生成配置", "backend")?;
    params.model = required_config_value(&params.model, "图片生成配置", "model")?;
    params.base_url = required_config_value(&params.base_url, "图片生成配置", "base_url")?;
    params.api_key_encrypted =
        normalize_image_backend_api_key(&params.backend, params.api_key_encrypted)?;
    params.max_concurrent = positive_i32(params.max_concurrent, "图片配置 max_concurrent")?;
    params.default_width = positive_i32(params.default_width, "图片配置 default_width")?;
    params.default_height = positive_i32(params.default_height, "图片配置 default_height")?;
    Ok(params)
}

pub fn validate_image_config_model(
    model: &image_gen_config::Model,
) -> Result<CreateImageConfigParams, AppError> {
    validate_image_config_params(CreateImageConfigParams {
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model
            .base_url
            .clone()
            .ok_or_else(|| AppError::InvalidArgument("图片生成配置缺少 base_url".into()))?,
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        max_concurrent: model.max_concurrent,
        default_width: model.default_width,
        default_height: model.default_height,
    })
}

fn normalize_image_backend_api_key(
    backend: &str,
    api_key_encrypted: Option<String>,
) -> Result<Option<String>, AppError> {
    let capabilities = image_provider_capabilities(backend)
        .ok_or_else(|| AppError::InvalidArgument(format!("未知图片生成后端: {backend}")))?;
    if capabilities.requires_api_key {
        return Ok(normalize_optional_config_value(api_key_encrypted));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> CreateImageConfigParams {
        CreateImageConfigParams {
            name: "OpenAI".into(),
            backend: "openai-compatible-image".into(),
            base_url: "https://api.openai.com/v1".into(),
            api_key_encrypted: Some("sk-test".into()),
            model: "dall-e-3".into(),
            max_concurrent: 1,
            default_width: 1024,
            default_height: 1024,
        }
    }

    #[test]
    fn requires_base_url_and_positive_limits() {
        let mut missing_base_url = params();
        missing_base_url.base_url = " ".into();
        assert!(matches!(
            validate_image_config_params(missing_base_url),
            Err(AppError::InvalidArgument(_))
        ));

        type ConfigureLimit = fn(&mut CreateImageConfigParams);
        let cases: [(&str, ConfigureLimit); 3] = [
            ("max_concurrent", |params| params.max_concurrent = 0),
            ("default_width", |params| params.default_width = 0),
            ("default_height", |params| params.default_height = 0),
        ];
        for (field, configure) in cases {
            let mut params = params();
            configure(&mut params);
            assert_eq!(
                validate_image_config_params(params)
                    .expect_err("zero limit")
                    .to_string(),
                format!("参数无效：图片配置 {field} 必须大于 0")
            );
        }
    }

    #[test]
    fn accepts_supported_remote_backends_without_required_api_keys() {
        for backend in IMAGE_PROVIDER_CAPABILITIES
            .iter()
            .filter(|capabilities| capabilities.requires_api_key)
        {
            let mut params = params();
            params.backend = backend.backend.to_string();
            params.api_key_encrypted = None;
            assert_eq!(
                validate_image_config_params(params)
                    .expect("supported remote backend")
                    .api_key_encrypted,
                None
            );
        }
    }

    #[test]
    fn clears_api_keys_for_local_backends_and_rejects_unknown_backends() {
        for backend in IMAGE_PROVIDER_CAPABILITIES
            .iter()
            .filter(|capabilities| !capabilities.requires_api_key)
        {
            let mut params = params();
            params.backend = backend.backend.to_string();
            assert_eq!(
                validate_image_config_params(params)
                    .expect("supported local backend")
                    .api_key_encrypted,
                None
            );
        }

        let mut unsupported = params();
        unsupported.backend = "unsupported-image-backend".into();
        assert!(matches!(
            validate_image_config_params(unsupported),
            Err(AppError::InvalidArgument(_))
        ));
    }

    #[test]
    fn image_backend_capabilities_have_one_canonical_rule_set() {
        for capabilities in IMAGE_PROVIDER_CAPABILITIES {
            assert_eq!(
                image_backend_requires_api_key(capabilities.backend),
                capabilities.requires_api_key
            );
            assert_eq!(
                image_backend_supports_reference_images(capabilities.backend),
                capabilities.supports_reference_images
            );
        }
        assert!(image_provider_capabilities("unknown-image-backend").is_none());

        assert_eq!(
            image_backend_api_key("openai-compatible-image", Some("  secret  "))
                .expect("remote backend key"),
            "secret"
        );
        assert!(image_backend_api_key("openai-compatible-image", None).is_err());
        let error = image_backend_api_key("openai-compatible-image", Some("  "))
            .expect_err("blank remote backend key");
        assert_eq!(
            crate::config::app_error_plain_message(error),
            "图片生成配置缺少 api_key_encrypted"
        );
        assert_eq!(
            image_backend_api_key("stable-diffusion", None).expect("local backend key"),
            ""
        );
    }
}
