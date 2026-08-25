use super::mapping::embedding_config_summary;
use crate::actions::config_records::shared::require_config_admin;
use crate::{AppState, actions::app_error_message};
use a4n_api_contract::config::{
    CreateEmbeddingInput, EmbeddingConfigSummary, UpdateEmbeddingInput,
};
use a4n_core::error::AppError;

pub async fn list_embedding_config(
    state: &AppState,
) -> Result<Vec<EmbeddingConfigSummary>, String> {
    let configs = state
        .orchestrator()
        .list_embedding_configs()
        .await
        .map_err(app_error_message)?;
    Ok(configs.iter().map(embedding_config_summary).collect())
}

pub async fn create_embedding_config(
    state: &AppState,
    caller_user_id: i64,
    input: CreateEmbeddingInput,
) -> Result<EmbeddingConfigSummary, AppError> {
    require_config_admin(state, caller_user_id).await?;
    let config = state
        .orchestrator()
        .create_embedding_config_validated(input)
        .await?;
    Ok(embedding_config_summary(&config))
}

pub async fn update_embedding_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
    input: UpdateEmbeddingInput,
) -> Result<EmbeddingConfigSummary, AppError> {
    require_config_admin(state, caller_user_id).await?;
    let updated = state
        .orchestrator()
        .update_embedding_config_validated(config_id, input)
        .await?;
    Ok(embedding_config_summary(&updated))
}

pub async fn set_default_embedding_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), AppError> {
    require_config_admin(state, caller_user_id).await?;
    state
        .orchestrator()
        .set_default_embedding_config_validated(config_id)
        .await
}

pub async fn delete_embedding_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), AppError> {
    require_config_admin(state, caller_user_id).await?;
    state
        .orchestrator()
        .delete_embedding_config_if_allowed(config_id)
        .await
}
