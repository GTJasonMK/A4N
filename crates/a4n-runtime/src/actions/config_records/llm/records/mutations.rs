use super::super::contracts::{
    CreateLlmInput, LlmConfigSummary, UpdateLlmInput, llm_config_summary,
};
use crate::AppState;
use crate::actions::config_records::shared::require_config_admin;
use a4n_core::error::AppError;
use a4n_llm::config::{CreateLlmConfigInput, UpdateLlmConfigInput};

pub async fn create_llm_config(
    state: &AppState,
    caller_user_id: i64,
    input: CreateLlmInput,
) -> Result<LlmConfigSummary, AppError> {
    require_config_admin(state, caller_user_id).await?;
    let create_input = CreateLlmConfigInput {
        name: &input.name,
        provider: &input.provider,
        base_url: &input.base_url,
        api_key_encrypted: input.api_key_encrypted.as_deref(),
        model: &input.model,
        max_concurrent: input.max_concurrent,
        timeout_secs: input.timeout_secs,
        max_context_tokens: input.max_context_tokens,
        max_output_tokens: input.max_output_tokens,
        extra_body_json: input.extra_body_json,
    };
    let config = state
        .orchestrator()
        .create_llm_config_validated(create_input)
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
    let update_input = UpdateLlmConfigInput {
        name: &input.name,
        provider: &input.provider,
        base_url: &input.base_url,
        api_key_encrypted: input
            .api_key_encrypted
            .as_ref()
            .map(|value| value.as_deref()),
        model: &input.model,
        max_concurrent: input.max_concurrent,
        timeout_secs: input.timeout_secs,
        max_context_tokens: input.max_context_tokens,
        max_output_tokens: input.max_output_tokens,
        extra_body_json: input.extra_body_json,
    };
    let config = state
        .orchestrator()
        .update_llm_config_validated(config_id, update_input)
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
