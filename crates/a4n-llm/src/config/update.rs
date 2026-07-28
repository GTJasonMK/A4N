use a4n_core::{
    config::{normalize_config_name, positive_i32, required_config_value},
    error::AppError,
    models::ai::llm_config,
};

use super::{
    contracts::{NormalizedLlmConfig, UpdateLlmConfigInput},
    limits::ensure_output_tokens_below_context,
    validation::{normalize_llm_base_url, normalize_llm_request_options_json},
};

mod resolution;

use resolution::resolve_update_llm_config_input;

pub fn normalize_update_llm_config_input(
    cfg: &llm_config::Model,
    input: UpdateLlmConfigInput<'_>,
) -> Result<NormalizedLlmConfig, AppError> {
    let resolution::ResolvedLlmConfigUpdate {
        name,
        provider,
        base_url,
        api_key_encrypted,
        model,
        max_concurrent,
        timeout_secs,
        max_context_tokens,
        max_output_tokens,
        request_options_json,
    } = resolve_update_llm_config_input(cfg, input);
    let name = normalize_config_name(&name, "LLM配置")?;
    let provider = required_config_value(&provider, "LLM配置", "provider")?;
    let base_url = normalize_llm_base_url(&base_url, api_key_encrypted.as_deref())?;
    let model = required_config_value(&model, "LLM配置", "model")?;
    let max_concurrent = positive_i32(max_concurrent, "LLM配置 max_concurrent")?;
    let timeout_secs = positive_i32(timeout_secs, "LLM配置 timeout_secs")?;
    let max_context_tokens = positive_i32(max_context_tokens, "LLM配置 max_context_tokens")?;
    let max_output_tokens = positive_i32(max_output_tokens, "LLM配置 max_output_tokens")?;
    let request_options_json = normalize_llm_request_options_json(request_options_json)?;
    ensure_output_tokens_below_context(max_output_tokens, max_context_tokens)?;
    Ok(NormalizedLlmConfig {
        name,
        provider,
        base_url,
        api_key_encrypted,
        model,
        max_concurrent,
        timeout_secs,
        max_context_tokens,
        max_output_tokens,
        request_options_json,
    })
}
