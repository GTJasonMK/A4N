use super::super::shared::validate_runtime_snapshot_config_id;
use crate::actions::request_preview::{
    ModelRequestPreview, image_provider_request_step, model_request_preview, provider_request_step,
};
use crate::{AppState, actions::app_error_message};
use a4n_api_contract::config::ImageConfigForConnectivityTest;
use a4n_error::AppError;
use a4n_image_contract::ImageGenerationRequest;
use a4n_provider_contract::ImageProviderConfig;
use a4n_queue::priority::{TaskKind as QueueTaskKind, TaskPriority};

const IMAGE_CONNECTIVITY_TEST_PROMPT: &str = "test connectivity, draw a tiny dot";

pub async fn test_image_config(
    state: &AppState,
    user_id: i64,
    config_id: i64,
) -> Result<(), String> {
    let config = resolve_image_config_for_connectivity_test(state, user_id, config_id).await?;
    let request = build_image_connectivity_test_request(&config)?;
    let profile = state.runtime_profile();
    let image_client = state.capabilities().image.clone();
    state
        .provider_runtime()
        .queue_run_tracked(
            QueueTaskKind::ImageGeneration,
            TaskPriority::High,
            None,
            || async move {
                crate::sidecar_clients::generate_image(
                    profile,
                    image_client,
                    &config,
                    request,
                    None,
                    None,
                    None,
                    None,
                )
                .await
                .map(|_| ())
                .map_err(AppError::ConfigError)
            },
        )
        .await
        .map_err(|e| e.to_string())
}

pub async fn preview_test_image_config_request(
    state: &AppState,
    user_id: i64,
    config_id: i64,
) -> Result<ModelRequestPreview, String> {
    let config = resolve_image_config_for_connectivity_test(state, user_id, config_id).await?;
    let request = build_image_connectivity_test_request(&config)?;
    if state.runtime_profile().uses_sidecars() {
        let provider_request = crate::sidecar_clients::preview_image_provider_request(
            state.capabilities().image.clone(),
            &config,
            request,
            None,
            None,
        )
        .await?;
        return Ok(model_request_preview(
            "image_config_connectivity_test",
            "图片配置测试请求预览",
            vec![provider_request_step(
                "image_connectivity_test",
                "图片连接测试 provider 请求",
                "image_provider_request",
                None,
                "ready",
                Some(
                    "由 image sidecar 生成真实 provider 请求体；固定连接测试请求，不包含参考图"
                        .to_string(),
                ),
                &provider_request,
            )?],
        ));
    }
    Ok(model_request_preview(
        "image_config_connectivity_test",
        "图片配置测试请求预览",
        vec![image_provider_request_step(
            "image_connectivity_test",
            "图片连接测试 provider 请求",
            None,
            "ready",
            Some("固定连接测试请求，不包含参考图".to_string()),
            &config.backend,
            request,
        )?],
    ))
}

async fn resolve_image_config_for_connectivity_test(
    state: &AppState,
    user_id: i64,
    config_id: i64,
) -> Result<ImageProviderConfig, String> {
    if let Some(config) = crate::runtime_config::current_image_provider_config_for_id(None) {
        validate_runtime_snapshot_config_id("image", config.id, config_id)?;
        return Ok(config);
    }
    state
        .config_read_application()
        .image_config_for_connectivity_test(user_id, config_id)
        .await
        .map(image_provider_config_from_connectivity_config)
        .map_err(app_error_message)
}

fn image_provider_config_from_connectivity_config(
    config: ImageConfigForConnectivityTest,
) -> ImageProviderConfig {
    ImageProviderConfig {
        id: config.id,
        name: config.name,
        backend: config.backend,
        base_url: config.base_url,
        api_key: config.api_key,
        model: config.model,
        max_concurrent: config.max_concurrent,
        supported_sizes_json: config.supported_sizes_json,
    }
}

fn build_image_connectivity_test_request(
    config: &ImageProviderConfig,
) -> Result<ImageGenerationRequest, String> {
    let sizes = a4n_config::image_sizes_from_json(&config.supported_sizes_json)
        .map_err(|error| error.to_string())?;
    let size = sizes[0];
    Ok(ImageGenerationRequest {
        prompt: IMAGE_CONNECTIVITY_TEST_PROMPT.to_string(),
        width: size.width,
        height: size.height,
        model: config.model.clone(),
        reference_images: Vec::new(),
    })
}
