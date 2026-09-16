use super::super::shared::validate_runtime_snapshot_config_id;
use crate::actions::request_preview::{
    ModelRequestPreview, model_request_preview, provider_request_step,
};
use crate::{AppState, actions::app_error_message};
use a4n_domain::cost::{
    ModelCapability, ModelUsageMeter, ModelUsageRecordCompletion, ModelUsageRecordStart,
};
use a4n_embedding_provider_contract::EMBEDDING_CONNECTIVITY_TEST_TEXT;
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
    let cost_record = state
        .provider_runtime()
        .start_model_usage(ModelUsageRecordStart {
            request_id: uuid::Uuid::new_v4().to_string(),
            user_id: Some(user_id),
            project_id: None,
            root_project_id: None,
            workflow_run_id: None,
            generation_run_id: None,
            workflow: "embedding_connectivity_test".to_string(),
            step_key: "connectivity".to_string(),
            call_key: None,
            source: Some("connection_test".to_string()),
            capability: ModelCapability::Embedding,
            provider_config_id: Some(config.id),
            provider: Some(config.backend.clone()),
            model: Some(config.model.clone()),
            pricing: config.pricing.clone(),
        })
        .await
        .map_err(|error| error.to_string())?;
    let result = if state.runtime_profile().uses_sidecars() {
        let client = state.capabilities().embedding.clone();
        state
            .provider_runtime()
            .queue_run_tracked(
                QueueTaskKind::EmbeddingCall,
                TaskPriority::High,
                None,
                || async move {
                    crate::sidecar_clients::test_embedding_config_via_sidecar(client, &config)
                        .await
                        .map_err(a4n_error::AppError::from)
                },
            )
            .await
    } else {
        let provider = crate::dynamic_embedding::make_provider_from_model(
            &config,
            state.embedding_models_dir(),
        )
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
    };
    let status = if result.is_ok() {
        "completed"
    } else {
        "failed"
    };
    state
        .provider_runtime()
        .finish_model_usage(
            cost_record,
            ModelUsageRecordCompletion {
                status: status.to_string(),
                meter: ModelUsageMeter {
                    input_tokens: Some(1),
                    estimated: true,
                    ..Default::default()
                },
                error_kind: result.as_ref().err().map(|_| "provider".to_string()),
                error_message: result.as_ref().err().map(ToString::to_string),
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    result.map_err(|error| error.to_string())
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
        .config_read_application()
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
