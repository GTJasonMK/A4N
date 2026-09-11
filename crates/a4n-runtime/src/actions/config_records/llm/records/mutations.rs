use super::super::contracts::{CreateLlmInput, LlmConfigSummary, UpdateLlmInput};
use crate::AppState;
use a4n_error::AppError;

pub async fn create_llm_config(
    state: &AppState,
    caller_user_id: i64,
    input: CreateLlmInput,
) -> Result<LlmConfigSummary, AppError> {
    state
        .config_application()
        .create_llm(caller_user_id, input)
        .await
}

pub async fn update_llm_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
    input: UpdateLlmInput,
) -> Result<LlmConfigSummary, AppError> {
    state
        .config_application()
        .update_llm(caller_user_id, config_id, input)
        .await
}

pub async fn set_default_llm_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), AppError> {
    state
        .config_application()
        .set_default_llm(caller_user_id, config_id)
        .await
}

pub async fn delete_llm_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), AppError> {
    state
        .config_application()
        .delete_llm(caller_user_id, config_id)
        .await
}
