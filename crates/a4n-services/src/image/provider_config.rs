use a4n_core::error::AppError;
use a4n_core::models::ai::image_gen_config;
use a4n_engine::image::{ImageProvider, pick_image_provider};

use crate::config::validate_image_config_model;

pub fn configured_image_provider(
    config: &image_gen_config::Model,
) -> Result<Box<dyn ImageProvider>, AppError> {
    let params = validate_image_config_model(config)?;
    let api_key = image_provider_api_key(&params.backend, params.api_key_encrypted.as_deref())?;
    pick_image_provider(
        &params.backend,
        &params.base_url,
        api_key,
        params.provider_options_json.as_ref(),
    )
}

fn image_provider_api_key<'a>(
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

fn image_backend_requires_api_key(backend: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn image_config(backend: &str, api_key: Option<&str>) -> image_gen_config::Model {
        let now = chrono::Utc::now().naive_utc();
        image_gen_config::Model {
            id: 1,
            name: "primary".to_string(),
            backend: backend.to_string(),
            base_url: Some("http://localhost:3000".to_string()),
            api_key_encrypted: api_key.map(str::to_string),
            model: "image-model".to_string(),
            max_concurrent: 1,
            default_width: 1024,
            default_height: 1024,
            provider_options_json: None,
            is_default: true,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn configured_image_provider_rejects_api_backend_without_api_key() {
        let Err(err) = configured_image_provider(&image_config("openai-compatible-image", None))
        else {
            panic!("api image backend should require api key");
        };

        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(err.to_string().contains("api_key_encrypted"));
    }

    #[test]
    fn configured_image_provider_accepts_local_backend_without_api_key() {
        let provider = configured_image_provider(&image_config("stable-diffusion", None));

        assert!(provider.is_ok());
    }

    #[test]
    fn reference_image_support_is_limited_to_image_input_backends() {
        assert!(image_backend_supports_reference_images(
            "openai-compatible-image-edit"
        ));
        assert!(image_backend_supports_reference_images("openai-chat-image"));
        assert!(image_backend_supports_reference_images("cliproxy-image"));
        assert!(!image_backend_supports_reference_images(
            "openai-compatible-image"
        ));
        assert!(!image_backend_supports_reference_images("stable-diffusion"));
        assert!(!image_backend_supports_reference_images("comfyui"));
    }
}
