use super::super::shared::validate_runtime_snapshot_config_id;
use crate::actions::request_preview::{
    ModelRequestPreview, image_provider_request_step, model_request_preview, provider_request_step,
};
use crate::{AppState, actions::app_error_message};
use a4n_core::{error::AppError, models::ai::image_gen_config};
use a4n_image::ImageGenerationRequest;
use a4n_orchestrator::config::configured_image_provider;
use a4n_queue::priority::{TaskKind as QueueTaskKind, TaskPriority};

const IMAGE_CONNECTIVITY_TEST_PROMPT: &str = "test connectivity, draw a tiny dot";

pub async fn test_image_config(state: &AppState, config_id: i64) -> Result<bool, String> {
    let config = resolve_image_config_for_connectivity_test(state, config_id).await?;
    let request = build_image_connectivity_test_request(&config);
    if state.runtime_profile().uses_sidecars() {
        let capabilities = state.capabilities().clone();
        let config = config.clone();
        state
            .task_queue()
            .queue_run_tracked(
                QueueTaskKind::ImageGeneration,
                TaskPriority::High,
                None,
                || async move {
                    crate::sidecar_clients::probe_image_config(&capabilities, &config, request)
                        .await
                        .map_err(AppError::ConfigError)
                },
            )
            .await
            .map_err(|e| e.to_string())?;
        return Ok(true);
    }

    let provider = configured_image_provider(&config).map_err(|e| e.to_string())?;
    state
        .task_queue()
        .queue_run_tracked(
            QueueTaskKind::ImageGeneration,
            TaskPriority::High,
            None,
            || async move { provider.generate(request).await.map(|_| ()) },
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(true)
}

pub async fn preview_test_image_config_request(
    state: &AppState,
    config_id: i64,
) -> Result<ModelRequestPreview, String> {
    let config = resolve_image_config_for_connectivity_test(state, config_id).await?;
    let request = build_image_connectivity_test_request(&config);
    if state.runtime_profile().uses_sidecars() {
        let provider_request = crate::sidecar_clients::preview_image_provider_request(
            state.capabilities(),
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
    config_id: i64,
) -> Result<image_gen_config::Model, String> {
    if let Some(config) = crate::runtime_config::current_image_model() {
        validate_runtime_snapshot_config_id("image", config.id, config_id)?;
        return Ok(config);
    }
    state
        .settings()
        .image_config_record_by_id(config_id)
        .await
        .map_err(app_error_message)
}

fn build_image_connectivity_test_request(
    config: &image_gen_config::Model,
) -> ImageGenerationRequest {
    ImageGenerationRequest {
        prompt: IMAGE_CONNECTIVITY_TEST_PROMPT.to_string(),
        width: config.default_width,
        height: config.default_height,
        model: config.model.clone(),
        reference_images: Vec::new(),
    }
}
