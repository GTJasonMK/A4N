use std::sync::Arc;

#[cfg(feature = "embedded-media-providers")]
use a4n_config::image_backend_api_key;
use a4n_error::AppError;
use a4n_image_client::ImageClient;
use a4n_image_contract::ImageGenerationRequest;
use a4n_image_provider_contract::{ImageGenerationResult, ImageProvider};
#[cfg(feature = "embedded-media-providers")]
use a4n_image_providers::pick_image_provider;
use a4n_provider_contract::ImageProviderConfig;

use crate::state::RuntimeProfile;

use super::super::super::missing_sidecar_capability_message;
use super::super::contracts::image_provider_error_with_config;
#[cfg(feature = "embedded-media-providers")]
use super::super::contracts::validated_image_provider_config;
use super::sidecar::generate_image_via_sidecar;

fn configured_image_provider(
    config: &ImageProviderConfig,
) -> Result<Box<dyn ImageProvider>, AppError> {
    #[cfg(feature = "embedded-media-providers")]
    {
        let params = validated_image_provider_config(config).map_err(AppError::InvalidArgument)?;
        let api_key = image_backend_api_key(&params.backend, params.api_key_encrypted.as_deref())?;
        Ok(pick_image_provider(
            &params.backend,
            &params.base_url,
            api_key,
        )?)
    }
    #[cfg(not(feature = "embedded-media-providers"))]
    {
        let _ = config;
        Err(AppError::ConfigError(
            "图片 provider 未启用：请使用 image sidecar，或用 embedded-media-providers feature 构建"
                .to_string(),
        ))
    }
}

pub(crate) async fn generate_image(
    profile: RuntimeProfile,
    client: Option<Arc<dyn ImageClient>>,
    config: &ImageProviderConfig,
    request: ImageGenerationRequest,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
    workflow_run_id: Option<i64>,
    progress: Option<&(dyn Fn(&str) + Send + Sync)>,
) -> Result<ImageGenerationResult, String> {
    if profile.uses_sidecars() {
        let client = client.ok_or_else(|| missing_sidecar_capability_message("image"))?;
        return generate_image_via_sidecar(
            client,
            config,
            request,
            project_id,
            root_project_id,
            workflow_run_id,
            progress,
        )
        .await;
    }

    let provider = configured_image_provider(config).map_err(|e| e.to_string())?;
    provider
        .generate(request)
        .await
        .map_err(|e| image_provider_error_with_config(config, e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image_config(backend: &str, api_key: Option<&str>) -> ImageProviderConfig {
        ImageProviderConfig {
            id: 1,
            name: "primary".to_string(),
            backend: backend.to_string(),
            base_url: Some("http://localhost:3000".to_string()),
            api_key: api_key.map(str::to_string),
            model: "image-model".to_string(),
            max_concurrent: 1,
            supported_sizes_json: serde_json::json!([{"width": 1024, "height": 1024}]),
            pricing: Default::default(),
        }
    }

    #[cfg(feature = "embedded-media-providers")]
    #[test]
    fn configured_image_provider_rejects_api_backend_without_api_key() {
        let Err(err) = configured_image_provider(&image_config("openai-compatible-image", None))
        else {
            panic!("api image backend should require api key");
        };

        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(err.to_string().contains("api_key_encrypted"));
    }

    #[cfg(feature = "embedded-media-providers")]
    #[test]
    fn configured_image_provider_accepts_local_backend_without_api_key() {
        let provider = configured_image_provider(&image_config("stable-diffusion", None));

        assert!(provider.is_ok());
    }

    #[cfg(not(feature = "embedded-media-providers"))]
    #[test]
    fn configured_image_provider_requires_sidecar_without_embedded_feature() {
        let Err(err) = configured_image_provider(&image_config("stable-diffusion", None)) else {
            panic!("minimal build must not construct an embedded image provider");
        };

        assert!(matches!(err, AppError::ConfigError(_)));
        assert!(err.to_string().contains("请使用 image sidecar"));
    }
}
