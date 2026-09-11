use super::super::shared::validate_runtime_snapshot_config_id;
use crate::actions::request_preview::{
    ModelRequestPreview, model_request_preview, provider_request_step,
};
use crate::{AppState, actions::app_error_message};
use a4n_embedding::provider::EMBEDDING_CONNECTIVITY_TEST_TEXT;
use a4n_provider_contract::EmbeddingProviderConfig;
use a4n_queue::priority::{TaskKind as QueueTaskKind, TaskPriority};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
struct EmbeddingConnectivityPreviewRequest {
    backend: String,
    model: String,
    dimension: i32,
    input: Vec<String>,
}

pub async fn test_embedding_config(
    state: &AppState,
    user_id: i64,
    config_id: i64,
) -> Result<(), String> {
    let config = resolve_embedding_config_for_connectivity_test(state, user_id, config_id).await?;
    if state.runtime_profile().uses_sidecars() {
        let capabilities = state.capabilities().clone();
        state
            .provider_runtime()
            .queue_run_tracked(
                QueueTaskKind::EmbeddingCall,
                TaskPriority::High,
                None,
                || async move {
                    crate::sidecar_clients::test_embedding_config_via_sidecar(
                        &capabilities,
                        &config,
                    )
                    .await
                },
            )
            .await
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    let provider =
        crate::dynamic_embedding::make_provider_from_model(&config, state.embedding_models_dir())
            .map_err(|e| e.to_string())?;
    state
        .provider_runtime()
        .queue_run_tracked(
            QueueTaskKind::EmbeddingCall,
            TaskPriority::High,
            None,
            || async move {
                provider
                    .test_connection()
                    .await
                    .map_err(a4n_error::AppError::from)
            },
        )
        .await
        .map_err(|e| e.to_string())
}

pub async fn preview_test_embedding_config_request(
    state: &AppState,
    user_id: i64,
    config_id: i64,
) -> Result<ModelRequestPreview, String> {
    let config = resolve_embedding_config_for_connectivity_test(state, user_id, config_id).await?;
    let request = build_embedding_connectivity_test_request(&config);
    Ok(model_request_preview(
        "embedding_config_connectivity_test",
        "Embedding 配置测试请求预览",
        vec![provider_request_step(
            "embedding_connectivity_test",
            "Embedding 连接测试请求",
            "embedding",
            None,
            "ready",
            Some("固定连接测试输入：test".to_string()),
            &request,
        )?],
    ))
}

async fn resolve_embedding_config_for_connectivity_test(
    state: &AppState,
    user_id: i64,
    config_id: i64,
) -> Result<EmbeddingProviderConfig, String> {
    if let Some(config) = crate::runtime_config::current_embedding_provider_config() {
        validate_runtime_snapshot_config_id("embedding", config.id, config_id)?;
        return Ok(config);
    }
    state
        .config_application()
        .embedding_config_for_connectivity_test(user_id, config_id)
        .await
        .map_err(app_error_message)
}

fn build_embedding_connectivity_test_request(
    config: &a4n_provider_contract::EmbeddingProviderConfig,
) -> EmbeddingConnectivityPreviewRequest {
    EmbeddingConnectivityPreviewRequest {
        backend: config.backend.clone(),
        model: config.model.clone(),
        dimension: config.dimension,
        input: vec![EMBEDDING_CONNECTIVITY_TEST_TEXT.to_string()],
    }
}
