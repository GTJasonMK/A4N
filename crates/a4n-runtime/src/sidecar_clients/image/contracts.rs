use a4n_core::config::validate_image_config_model;
use a4n_core::models::ai::image_gen_config;
use a4n_image::ImageGenerationRequest;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(super) fn image_provider_payload(
    config: &image_gen_config::Model,
    request: ImageGenerationRequest,
) -> Result<ImageProviderPayload, String> {
    let params = validate_image_config_model(config).map_err(|err| err.to_string())?;
    let api_key =
        image_sidecar_api_key(&params.backend, params.api_key_encrypted.as_deref())?.to_string();
    Ok(ImageProviderPayload {
        backend: params.backend.clone(),
        base_url: params.base_url.clone(),
        api_key,
        request,
    })
}

fn image_sidecar_api_key<'a>(backend: &str, api_key: Option<&'a str>) -> Result<&'a str, String> {
    if matches!(
        backend,
        "openai-compatible-image"
            | "openai-compatible-image-edit"
            | "openai-chat-image"
            | "cliproxy-image"
    ) {
        return api_key
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "图片生成配置缺少 api_key_encrypted".to_string());
    }
    Ok(api_key.unwrap_or(""))
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ImageProviderProbeOutput {
    pub(super) ok: bool,
    pub(super) request_preview: Value,
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
