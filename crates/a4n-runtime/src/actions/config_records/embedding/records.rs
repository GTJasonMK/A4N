use super::contracts::{
    CreateEmbeddingInput, EmbeddingConfigSummary, UpdateEmbeddingInput, embedding_config_summary,
};
use super::mapping::{embedding_create_params, embedding_update_params};
use crate::actions::config_records::shared::require_config_admin;
use crate::{AppState, actions::app_error_message};

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
    input: CreateEmbeddingInput,
) -> Result<EmbeddingConfigSummary, String> {
    let params = embedding_create_params(input)?;
    let config = state
        .orchestrator()
        .create_embedding_config_validated(params)
        .await
        .map_err(app_error_message)?;
    Ok(embedding_config_summary(&config))
}

pub async fn update_embedding_config(
    state: &AppState,
    config_id: i64,
    input: UpdateEmbeddingInput,
) -> Result<EmbeddingConfigSummary, String> {
    let existing = state
        .orchestrator()
        .embedding_config_by_id(config_id)
        .await
        .map_err(app_error_message)?;
    let params = embedding_update_params(&existing, input)?;
    let updated = state
        .orchestrator()
        .update_embedding_config_validated(config_id, params)
        .await
        .map_err(app_error_message)?;
    Ok(embedding_config_summary(&updated))
}

pub async fn set_default_embedding_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), String> {
    require_config_admin(state, caller_user_id).await?;
    state
        .orchestrator()
        .set_default_embedding_config_validated(config_id)
        .await
        .map_err(app_error_message)
}

pub async fn delete_embedding_config(state: &AppState, config_id: i64) -> Result<(), String> {
    state
        .orchestrator()
        .delete_embedding_config_if_allowed(config_id)
        .await
        .map_err(app_error_message)
}
