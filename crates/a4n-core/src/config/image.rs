use a4n_domain::{ImageSize, validate_image_sizes};
use serde_json::Value;

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
    pub supported_sizes: Vec<ImageSize>,
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
    validate_image_sizes(&params.supported_sizes).map_err(AppError::InvalidArgument)?;
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
        supported_sizes: image_sizes_from_json(&model.supported_sizes_json)?,
    })
}

pub fn image_sizes_from_json(value: &Value) -> Result<Vec<ImageSize>, AppError> {
    let sizes = serde_json::from_value::<Vec<ImageSize>>(value.clone()).map_err(|error| {
        AppError::InvalidArgument(format!("图片配置 supported_sizes 无效: {error}"))
    })?;
    validate_image_sizes(&sizes).map_err(AppError::InvalidArgument)?;
    Ok(sizes)
}

pub fn image_sizes_to_json(sizes: &[ImageSize]) -> Result<Value, AppError> {
    validate_image_sizes(sizes).map_err(AppError::InvalidArgument)?;
    serde_json::to_value(sizes).map_err(|error| {
        AppError::InvalidArgument(format!("图片配置 supported_sizes 无效: {error}"))
    })
}

pub fn ensure_image_size_supported(
    sizes: &[ImageSize],
    width: i32,
    height: i32,
) -> Result<(), AppError> {
    if sizes
        .iter()
        .any(|size| size.width == width && size.height == height)
    {
        return Ok(());
    }
    Err(AppError::InvalidArgument(format!(
        "图片尺寸 {}x{} 不在配置 supported_sizes 白名单中",
        width, height
    )))
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
            supported_sizes: vec![ImageSize::new(1024, 1024)],
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

        let mut invalid_concurrency = params();
        invalid_concurrency.max_concurrent = 0;
        assert_eq!(
            validate_image_config_params(invalid_concurrency)
                .expect_err("zero concurrency")
                .to_string(),
            "参数无效：图片配置 max_concurrent 必须大于 0"
        );
    }

    #[test]
    fn validates_supported_sizes() {
        let mut empty = params();
        empty.supported_sizes.clear();
        assert!(validate_image_config_params(empty).is_err());

        let mut duplicate = params();
        duplicate.supported_sizes.push(ImageSize::new(1024, 1024));
        assert!(validate_image_config_params(duplicate).is_err());

        let mut invalid = params();
        invalid.supported_sizes = vec![ImageSize::new(0, 1024)];
        assert!(validate_image_config_params(invalid).is_err());
    }

    #[test]
    fn validates_image_model_sizes_from_json() {
        let mut model = image_gen_config::Model {
            id: 1,
            name: "image".to_string(),
            backend: OPENAI_COMPATIBLE_IMAGE_BACKEND.to_string(),
            base_url: Some("https://example.test/v1".to_string()),
            api_key_encrypted: Some("key".to_string()),
            model: "image-model".to_string(),
            max_concurrent: 1,
            supported_sizes_json: serde_json::json!([{"width": 1024, "height": 1024}]),
            is_default: true,
            created_at: chrono::NaiveDateTime::MIN,
            updated_at: chrono::NaiveDateTime::MIN,
        };
        assert_eq!(
            validate_image_config_model(&model)
                .expect("valid model")
                .supported_sizes,
            vec![ImageSize::new(1024, 1024)]
        );
        model.supported_sizes_json = serde_json::json!([]);
        assert!(validate_image_config_model(&model).is_err());
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
