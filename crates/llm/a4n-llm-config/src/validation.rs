use a4n_domain::config::{normalize_config_name, positive_i32, required_config_value};
use a4n_error::AppError;
use serde_json::Value;

use a4n_domain::agent::valid_agent_protocol;
use a4n_llm_provider_contract::{
    LlmProtocolKind, normalize_extra_body_json, validate_prompt_cache_options,
};

use super::limits::ensure_output_tokens_below_context;

pub(super) fn normalize_llm_extra_body_json(
    provider: &str,
    value: Option<Value>,
) -> Result<Option<Value>, AppError> {
    normalize_extra_body_json(value)
        .and_then(|value| {
            validate_prompt_cache_options(provider, value.as_ref())?;
            Ok(value)
        })
        .map_err(|err| {
            AppError::InvalidArgument(format!(
                "LLM配置 extra_body_json 无效：{}",
                app_error_plain_message(err)
            ))
        })
}

pub(super) fn normalize_llm_base_url(
    value: &str,
    api_key: Option<&str>,
) -> Result<String, AppError> {
    a4n_domain::config::validate_http_base_url(value, "LLM配置", api_key)
        .map_err(AppError::InvalidArgument)
}

pub fn validate_llm_config(model: &super::contracts::LlmConfigSnapshot) -> Result<(), AppError> {
    if LlmProtocolKind::for_provider(&model.provider).is_none() {
        return Err(AppError::ConfigError(format!(
            "LLM 配置 {} 使用不支持的 provider: {}",
            model.name, model.provider
        )));
    }
    normalize_config_name(&model.name, "LLM配置").map_err(AppError::InvalidArgument)?;
    required_config_value(&model.provider, "LLM配置", "provider")
        .map_err(AppError::InvalidArgument)?;
    normalize_llm_base_url(&model.base_url, model.api_key_encrypted.as_deref())?;
    required_config_value(&model.model, "LLM配置", "model").map_err(AppError::InvalidArgument)?;
    positive_i32(model.max_concurrent, "LLM配置 max_concurrent")
        .map_err(AppError::InvalidArgument)?;
    positive_i32(model.timeout_secs, "LLM配置 timeout_secs").map_err(AppError::InvalidArgument)?;
    positive_i32(model.max_context_tokens, "LLM配置 max_context_tokens")
        .map_err(AppError::InvalidArgument)?;
    positive_i32(model.max_output_tokens, "LLM配置 max_output_tokens")
        .map_err(AppError::InvalidArgument)?;
    ensure_output_tokens_below_context(model.max_output_tokens, model.max_context_tokens)?;
    if !valid_agent_protocol(&model.agent_protocol) {
        return Err(AppError::ConfigError(format!(
            "LLM 配置 {} 使用不支持的 agent_protocol: {}",
            model.name, model.agent_protocol
        )));
    }
    normalize_llm_extra_body_json(&model.provider, model.extra_body_json.clone())?;
    Ok(())
}

fn app_error_plain_message(error: AppError) -> String {
    match error {
        AppError::InvalidArgument(message)
        | AppError::ConfigError(message)
        | AppError::LlmError(message)
        | AppError::Timeout(message) => message,
        error => error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_llm_base_url;

    #[test]
    fn remote_http_with_api_key_is_rejected_before_authorization_can_leak() {
        let error = normalize_llm_base_url("http://api.example/v1", Some("secret"))
            .expect_err("remote HTTP with a credential must be rejected");
        assert!(error.to_string().contains("必须使用 HTTPS"));
        assert!(error.to_string().contains("Authorization"));

        assert!(normalize_llm_base_url("http://127.0.0.1:11434/v1", Some("secret")).is_ok());
        assert!(normalize_llm_base_url("http://api.example/v1", None).is_ok());
        assert!(normalize_llm_base_url("https://api.example/v1", Some("secret")).is_ok());
    }
}
