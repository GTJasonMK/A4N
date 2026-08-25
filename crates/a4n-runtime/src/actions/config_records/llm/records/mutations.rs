use super::super::contracts::{
    CreateLlmInput, LlmConfigSummary, UpdateLlmInput, llm_config_summary,
};
use crate::AppState;
use crate::actions::config_records::shared::require_config_admin;
use a4n_core::error::AppError;

pub async fn create_llm_config(
    state: &AppState,
    caller_user_id: i64,
    input: CreateLlmInput,
) -> Result<LlmConfigSummary, AppError> {
    require_config_admin(state, caller_user_id).await?;
    let config = state
        .orchestrator()
        .create_llm_config_validated(input)
        .await?;
    Ok(llm_config_summary(&config))
}

pub async fn update_llm_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
    input: UpdateLlmInput,
) -> Result<LlmConfigSummary, AppError> {
    require_config_admin(state, caller_user_id).await?;
    let config = state
        .orchestrator()
        .update_llm_config_validated(config_id, input)
        .await?;
    Ok(llm_config_summary(&config))
}

pub async fn set_default_llm_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), AppError> {
    require_config_admin(state, caller_user_id).await?;
    state
        .orchestrator()
        .set_default_llm_config_validated(config_id)
        .await
}

pub async fn delete_llm_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), AppError> {
    require_config_admin(state, caller_user_id).await?;
    state
        .orchestrator()
        .delete_llm_config_if_allowed(config_id)
        .await
}
