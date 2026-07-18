use a4n_core::{
    config::{app_error_plain_message, normalize_config_name, positive_i32, required_config_value},
    error::AppError,
    models::ai::llm_config,
};
use serde_json::Value;

use crate::provider::normalize_request_options_json;

use super::limits::ensure_output_tokens_below_context;

pub(super) fn normalize_llm_request_options_json(
    value: Option<Value>,
) -> Result<Option<Value>, AppError> {
    normalize_request_options_json(value).map_err(|err| {
        AppError::InvalidArgument(format!(
            "LLM配置 request_options_json 无效：{}",
            app_error_plain_message(err)
        ))
    })
}

pub fn validate_llm_config_model(model: &llm_config::Model) -> Result<(), AppError> {
    normalize_config_name(&model.name, "LLM配置")?;
    required_config_value(&model.provider, "LLM配置", "provider")?;
    required_config_value(&model.base_url, "LLM配置", "base_url")?;
    required_config_value(&model.model, "LLM配置", "model")?;
    positive_i32(model.max_concurrent, "LLM配置 max_concurrent")?;
    positive_i32(model.timeout_secs, "LLM配置 timeout_secs")?;
    positive_i32(model.max_context_tokens, "LLM配置 max_context_tokens")?;
    positive_i32(model.max_output_tokens, "LLM配置 max_output_tokens")?;
    ensure_output_tokens_below_context(model.max_output_tokens, model.max_context_tokens)?;
    normalize_llm_request_options_json(model.request_options_json.clone())?;
    Ok(())
}
