use super::super::shared::{
    ModelRequestPreview, positive_i32_as_usize, validate_runtime_snapshot_config_id,
};
use crate::actions::request_preview::{model_request_preview, provider_request_step};
use crate::{AppState, actions::app_error_message};
use a4n_core::models::ai::embedding_config;
use a4n_queue::priority::{TaskPriority, TaskType as QueueTaskType};
use serde::Serialize;

const EMBEDDING_CONNECTIVITY_TEST_INPUT: &str = "test";

#[derive(Debug, Clone, Serialize)]
struct EmbeddingConnectivityPreviewRequest {
    backend: String,
    model: String,
    dimension: i32,
    input: Vec<String>,
}

pub async fn test_embedding_config(state: &AppState, config_id: i64) -> Result<bool, String> {
    let config = resolve_embedding_config_for_connectivity_test(state, config_id).await?;
    let expected_dimension = positive_i32_as_usize(config.dimension, "嵌入配置维度")?;
    if state.runtime_profile.uses_sidecars() {
        let capabilities = state.capabilities().clone();
        return state
            .task_queue
            .run_tracked(
                QueueTaskType::EmbeddingCall,
                TaskPriority::High,
                None,
                || async move {
                    crate::sidecar_clients::test_embedding_config_via_sidecar(
                        &capabilities,
                        &config,
                        EMBEDDING_CONNECTIVITY_TEST_INPUT,
                    )
                    .await
                },
            )
            .await
            .map_err(|e| e.to_string());
    }
    let provider = crate::dynamic_embedding::make_provider_from_model(
        &config,
        crate::dynamic_embedding::default_fastembed_cache_dir(&state.config),
    )
    .map_err(|e| e.to_string())?;
    let vectors = state
        .task_queue
        .run_tracked(
            QueueTaskType::EmbeddingCall,
            TaskPriority::High,
            None,
            || async move {
                provider
                    .embed(vec![EMBEDDING_CONNECTIVITY_TEST_INPUT.to_string()])
                    .await
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(vectors
        .first()
        .is_some_and(|vector| vector.len() == expected_dimension))
}

pub async fn preview_test_embedding_config_request(
    state: &AppState,
    config_id: i64,
) -> Result<ModelRequestPreview, String> {
    let config = resolve_embedding_config_for_connectivity_test(state, config_id).await?;
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
    config_id: i64,
) -> Result<embedding_config::Model, String> {
    if let Some(config) = crate::runtime_config::current_embedding_model() {
        validate_runtime_snapshot_config_id("embedding", config.id, config_id)?;
        return Ok(config);
    }
    state
        .orchestrator
        .embedding_config_by_id(config_id)
        .await
        .map_err(app_error_message)
}

fn build_embedding_connectivity_test_request(
    config: &embedding_config::Model,
) -> EmbeddingConnectivityPreviewRequest {
    EmbeddingConnectivityPreviewRequest {
        backend: config.backend.clone(),
        model: config.model.clone(),
        dimension: config.dimension,
        input: vec![EMBEDDING_CONNECTIVITY_TEST_INPUT.to_string()],
    }
}
