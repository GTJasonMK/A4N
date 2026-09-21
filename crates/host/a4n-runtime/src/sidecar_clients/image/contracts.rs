use a4n_config::{image_backend_api_key, image_sizes_from_json, validate_image_config_params};
use a4n_domain::image::CreateImageConfigParams;
use a4n_image_contract::{ImageGenerationRequest, ImageProviderPayload};
use a4n_provider_contract::ImageProviderConfig;

pub(super) use a4n_image_contract::ImageGenerationOutput;

pub(super) fn validated_image_provider_config(
    config: &ImageProviderConfig,
) -> Result<CreateImageConfigParams, String> {
    let base_url = config
        .base_url
        .clone()
        .ok_or_else(|| "图片生成配置缺少 base_url".to_string())?;
    let supported_sizes =
        image_sizes_from_json(&config.supported_sizes_json).map_err(|error| error.to_string())?;
    validate_image_config_params(CreateImageConfigParams {
        name: config.name.clone(),
        backend: config.backend.clone(),
        base_url,
        api_key_encrypted: config.api_key.clone(),
        model: config.model.clone(),
        max_concurrent: config.max_concurrent,
        supported_sizes,
    })
    .map_err(|error| error.to_string())
}

pub(super) fn image_provider_payload(
    config: &ImageProviderConfig,
    request: ImageGenerationRequest,
) -> Result<ImageProviderPayload, String> {
    let params = validated_image_provider_config(config)?;
    let api_key = image_backend_api_key(&params.backend, params.api_key_encrypted.as_deref())
        .map_err(|error| error.to_string())?
        .to_string();
    Ok(ImageProviderPayload {
        backend: params.backend,
        base_url: params.base_url,
        api_key,
        request,
    })
}

pub(super) fn image_provider_error_with_config(
    config: &ImageProviderConfig,
    error: String,
) -> String {
    format!(
        "{error}；实际使用图片配置：id={}，name={}，backend={}，model={}，has_api_key={}",
        config.id,
        config.name,
        config.backend,
        config.model,
        config
            .api_key
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
    )
}

#[cfg(test)]
mod strict_output_tests {
    use super::*;

    fn image_config() -> ImageProviderConfig {
        ImageProviderConfig {
            id: 1,
            name: "test-image".to_string(),
            backend: "openai-compatible-image".to_string(),
            base_url: Some("https://example.invalid/v1".to_string()),
            api_key: Some("test-key".to_string()),
            model: "image-model".to_string(),
            max_concurrent: 1,
            supported_sizes_json: serde_json::json!([{"width": 1024, "height": 1024}]),
            pricing: Default::default(),
        }
    }

    #[test]
    fn image_output_rejects_unknown_fields() {
        let error = serde_json::from_value::<ImageGenerationOutput>(serde_json::json!({
            "bytes_base64": "aW1n",
            "format": "png",
            "model": "image-model",
            "legacy_url": "https://example.com/image.png"
        }))
        .expect_err("image output must reject unknown fields");
        assert!(error.to_string().contains("legacy_url"));
    }

    #[test]
    fn provider_payload_rejects_invalid_provider_config() {
        let mut config = image_config();
        config.backend = "unknown-image-backend".to_string();

        let error = image_provider_payload(
            &config,
            ImageGenerationRequest {
                prompt: "test".to_string(),
                width: 1024,
                height: 1024,
                model: "image-model".to_string(),
                reference_images: Vec::new(),
            },
        )
        .expect_err("invalid image provider config must fail at the sidecar boundary");

        assert!(error.contains("未知图片生成后端"));
    }
}
