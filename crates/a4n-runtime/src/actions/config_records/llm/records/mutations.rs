use super::super::contracts::{
    CreateLlmInput, LlmConfigSummary, UpdateLlmInput, llm_config_summary,
};
use crate::{AppState, actions::app_error_message};
use a4n_llm::config::{CreateLlmConfigInput, UpdateLlmConfigInput};

pub async fn create_llm_config(
    state: &AppState,
    input: CreateLlmInput,
) -> Result<LlmConfigSummary, String> {
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
        .settings()
        .create_llm_config_validated(create_input)
        .await
        .map_err(app_error_message)?;
    Ok(llm_config_summary(&config))
}

pub async fn update_llm_config(
    state: &AppState,
    config_id: i64,
    input: UpdateLlmInput,
) -> Result<LlmConfigSummary, String> {
    let update_input = UpdateLlmConfigInput {
        name: input.name.as_deref(),
        provider: input.provider.as_deref(),
        base_url: input.base_url.as_deref(),
        api_key_encrypted: input
            .api_key_encrypted
            .as_ref()
            .map(|value| value.as_deref()),
        model: input.model.as_deref(),
        max_concurrent: input.max_concurrent,
        timeout_secs: input.timeout_secs,
        max_context_tokens: input.max_context_tokens,
        max_output_tokens: input.max_output_tokens,
        extra_body_json: input.extra_body_json,
    };
    let config = state
        .settings()
        .update_llm_config_validated(config_id, update_input)
        .await
        .map_err(app_error_message)?;
    Ok(llm_config_summary(&config))
}

pub async fn set_default_llm_config(state: &AppState, config_id: i64) -> Result<(), String> {
    state
        .settings()
        .set_default_llm_config_validated(config_id)
        .await
        .map(drop)
        .map_err(app_error_message)
}

pub async fn delete_llm_config(state: &AppState, config_id: i64) -> Result<(), String> {
    state
        .settings()
        .delete_llm_config_if_allowed(config_id)
        .await
        .map_err(app_error_message)
}
