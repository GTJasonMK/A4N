use super::{
    super::{
        contracts::{
            CreateLlmInput, LlmConfigSummary, UpdateLlmInput, llm_config_summary,
            positive_model_output_cap,
        },
        max_tokens::load_persisted_llm_max_tokens_config,
    },
    mapping::{llm_create_record_input, llm_update_record_input},
};
use crate::{AppState, actions::app_error_message};
use a4n_llm::config::{
    CreateLlmConfigInput, UpdateLlmConfigInput, normalize_create_llm_config_input,
    normalize_update_llm_config_input, validate_llm_config_model,
};

pub async fn create_llm_config(
    state: &AppState,
    input: CreateLlmInput,
) -> Result<LlmConfigSummary, String> {
    let normalized = normalize_create_llm_config_input(CreateLlmConfigInput {
        name: &input.name,
        provider: &input.provider,
        base_url: &input.base_url,
        api_key_encrypted: input.api_key_encrypted.as_deref(),
        model: &input.model,
        max_concurrent: input.max_concurrent,
        timeout_secs: input.timeout_secs,
        max_context_tokens: input.max_context_tokens,
        max_output_tokens: input.max_output_tokens,
        request_options_json: input.request_options_json,
    })
    .map_err(|error| error.to_string())?;
    let config = state
        .orchestrator
        .create_llm_config_record(llm_create_record_input(normalized))
        .await
        .map_err(app_error_message)?;
    Ok(llm_config_summary(&config))
}

pub async fn update_llm_config(
    state: &AppState,
    config_id: i64,
    input: UpdateLlmInput,
) -> Result<LlmConfigSummary, String> {
    let existing = state
        .orchestrator
        .llm_config_record_by_id(config_id)
        .await
        .map_err(app_error_message)?;
    let normalized = normalize_update_llm_config_input(
        &existing,
        UpdateLlmConfigInput {
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
            request_options_json: input.request_options_json,
        },
    )
    .map_err(|error| error.to_string())?;
    if existing.is_default {
        let model_cap = positive_model_output_cap(normalized.max_output_tokens)?;
        let max_tokens = load_persisted_llm_max_tokens_config(state).await?;
        max_tokens.validate_against_model_output_cap(model_cap)?;
    }
    let config = state
        .orchestrator
        .update_llm_config_record(existing, llm_update_record_input(normalized))
        .await
        .map_err(app_error_message)?;
    Ok(llm_config_summary(&config))
}

pub async fn set_default_llm_config(state: &AppState, config_id: i64) -> Result<(), String> {
    let target = state
        .orchestrator
        .llm_config_record_by_id(config_id)
        .await
        .map_err(app_error_message)?;
    validate_llm_config_model(&target).map_err(|error| error.to_string())?;
    let model_cap = positive_model_output_cap(target.max_output_tokens)?;
    let max_tokens = load_persisted_llm_max_tokens_config(state).await?;
    max_tokens.validate_against_model_output_cap(model_cap)?;
    state
        .orchestrator
        .set_default_llm_config_record(target)
        .await
        .map(drop)
        .map_err(app_error_message)
}

pub async fn delete_llm_config(state: &AppState, config_id: i64) -> Result<(), String> {
    let target = state
        .orchestrator
        .llm_config_record_by_id(config_id)
        .await
        .map_err(app_error_message)?;
    a4n_core::config::ensure_default_config_delete_allowed(" LLM 配置", target.is_default)
        .map_err(|error| error.to_string())?;
    state
        .orchestrator
        .delete_llm_config_record(config_id)
        .await
        .map_err(app_error_message)
}
