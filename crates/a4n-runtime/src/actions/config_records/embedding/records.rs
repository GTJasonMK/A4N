use super::contracts::{
    CreateEmbeddingInput, EmbeddingConfigSummary, UpdateEmbeddingInput, embedding_config_summary,
    embedding_create_record_input, embedding_update_record_input,
};
use crate::{AppState, actions::app_error_message};
use a4n_core::config::validate_embedding_config_model;

pub async fn list_embedding_config(
    state: &AppState,
) -> Result<Vec<EmbeddingConfigSummary>, String> {
    let configs = state
        .orchestrator
        .list_embedding_configs()
        .await
        .map_err(app_error_message)?;
    Ok(configs.iter().map(embedding_config_summary).collect())
}

pub async fn create_embedding_config(
    state: &AppState,
    input: CreateEmbeddingInput,
) -> Result<EmbeddingConfigSummary, String> {
    let record_input = embedding_create_record_input(input)?;
    let config = state
        .orchestrator
        .create_embedding_config_record(record_input)
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
        .orchestrator
        .embedding_config_by_id(config_id)
        .await
        .map_err(app_error_message)?;
    let record_input = embedding_update_record_input(&existing, input)?;
    let updated = state
        .orchestrator
        .update_embedding_config_record(existing, record_input)
        .await
        .map_err(app_error_message)?;
    Ok(embedding_config_summary(&updated))
}

pub async fn set_default_embedding_config(state: &AppState, config_id: i64) -> Result<(), String> {
    let target = state
        .orchestrator
        .embedding_config_by_id(config_id)
        .await
        .map_err(app_error_message)?;
    validate_embedding_config_model(&target).map_err(|err| err.to_string())?;
    state
        .orchestrator
        .set_default_embedding_config_record(target)
        .await
        .map(drop)
        .map_err(app_error_message)
}

pub async fn delete_embedding_config(state: &AppState, config_id: i64) -> Result<(), String> {
    let target = state
        .orchestrator
        .embedding_config_by_id(config_id)
        .await
        .map_err(app_error_message)?;
    a4n_core::config::ensure_default_config_delete_allowed(" Embedding 配置", target.is_default)
        .map_err(|err| err.to_string())?;
    state
        .orchestrator
        .delete_embedding_config_record(config_id)
        .await
        .map_err(app_error_message)
}
