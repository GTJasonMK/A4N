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

pub fn image_backend_requires_api_key(backend: &str) -> bool {
    matches!(
        backend,
        "openai-compatible-image"
            | "openai-compatible-image-edit"
            | "openai-chat-image"
            | "cliproxy-image"
    )
}

pub fn image_backend_supports_reference_images(backend: &str) -> bool {
    matches!(
        backend,
        "openai-compatible-image-edit" | "openai-chat-image" | "cliproxy-image"
    )
}

pub fn image_backend_api_key<'a>(
    backend: &str,
    api_key: Option<&'a str>,
) -> Result<&'a str, AppError> {
    if image_backend_requires_api_key(backend) {
        return api_key
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::InvalidArgument("图片生成配置缺少 api_key_encrypted".into()));
    }
    Ok(api_key.unwrap_or(""))
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
    if image_backend_requires_api_key(backend) {
        return Ok(normalize_optional_config_value(api_key_encrypted));
    }
    match backend {
        "stable-diffusion" | "comfyui" => Ok(None),
        other => Err(AppError::InvalidArgument(format!(
            "未知图片生成后端: {other}"
        ))),
    }
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
        for backend in [
            "openai-compatible-image",
            "openai-compatible-image-edit",
            "openai-chat-image",
            "cliproxy-image",
        ] {
            let mut params = params();
            params.backend = backend.to_string();
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
        for backend in ["stable-diffusion", "comfyui"] {
            let mut params = params();
            params.backend = backend.to_string();
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
        for backend in [
            "openai-compatible-image",
            "openai-compatible-image-edit",
            "openai-chat-image",
            "cliproxy-image",
        ] {
            assert!(image_backend_requires_api_key(backend));
        }
        for backend in ["stable-diffusion", "comfyui", "unknown-image-backend"] {
            assert!(!image_backend_requires_api_key(backend));
        }
        for backend in [
            "openai-compatible-image-edit",
            "openai-chat-image",
            "cliproxy-image",
        ] {
            assert!(image_backend_supports_reference_images(backend));
        }
        for backend in [
            "openai-compatible-image",
            "stable-diffusion",
            "comfyui",
            "unknown-image-backend",
        ] {
            assert!(!image_backend_supports_reference_images(backend));
        }

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
