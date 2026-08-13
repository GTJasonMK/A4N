use a4n_core::config::{image_backend_api_key, validate_image_config_model};
use a4n_core::models::ai::image_gen_config;
use a4n_image::ImageGenerationRequest;
use serde::{Deserialize, Serialize};

pub(super) fn image_provider_payload(
    config: &image_gen_config::Model,
    request: ImageGenerationRequest,
) -> Result<ImageProviderPayload, String> {
    let params = validate_image_config_model(config).map_err(|err| err.to_string())?;
    let api_key = image_backend_api_key(&params.backend, params.api_key_encrypted.as_deref())
        .map_err(|error| error.to_string())?
        .to_string();
    Ok(ImageProviderPayload {
        backend: params.backend.clone(),
        base_url: params.base_url.clone(),
        api_key,
        request,
    })
}

pub(super) fn image_provider_error_with_config(
    config: &image_gen_config::Model,
    error: String,
) -> String {
    format!(
        "{error}；实际使用图片配置：id={}，name={}，backend={}，model={}，has_api_key={}",
        config.id,
        config.name,
        config.backend,
        config.model,
        config
            .api_key_encrypted
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
    )
}

#[derive(Debug, Serialize)]
pub(super) struct ImageProviderPayload {
    backend: String,
    base_url: String,
    api_key: String,
    request: ImageGenerationRequest,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ImageGenerationOutput {
    pub(super) bytes_base64: String,
    pub(super) format: String,
    pub(super) model: String,
}

#[cfg(test)]
mod strict_output_tests {
    use super::*;

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
}
