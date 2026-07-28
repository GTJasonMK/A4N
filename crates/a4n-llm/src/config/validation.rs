use a4n_core::{
    config::{app_error_plain_message, normalize_config_name, positive_i32, required_config_value},
    error::AppError,
    models::ai::llm_config,
};
use serde_json::Value;
use url::{Host, Url};

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

pub(super) fn normalize_llm_base_url(
    value: &str,
    api_key: Option<&str>,
) -> Result<String, AppError> {
    let value = required_config_value(value, "LLM配置", "base_url")?;
    let url = Url::parse(&value).map_err(|error| {
        AppError::InvalidArgument(format!("LLM配置 base_url 不是有效 URL：{error}"))
    })?;
    if !matches!(url.scheme(), "http" | "https") || url.host().is_none() {
        return Err(AppError::InvalidArgument(
            "LLM配置 base_url 必须是有效的 http:// 或 https:// URL".to_string(),
        ));
    }
    let has_api_key = api_key.is_some_and(|key| !key.trim().is_empty());
    if url.scheme() == "http" && has_api_key && !is_loopback_host(url.host()) {
        return Err(AppError::InvalidArgument(
            "携带 API Key 的远程 LLM Base URL 必须使用 HTTPS；HTTP 重定向会丢失 Authorization，且明文传输可能泄露密钥。HTTP 仅允许 localhost 或回环地址"
                .to_string(),
        ));
    }
    Ok(value)
}

fn is_loopback_host(host: Option<Host<&str>>) -> bool {
    match host {
        Some(Host::Domain(host)) => {
            host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost")
        }
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    }
}

pub fn validate_llm_config_model(model: &llm_config::Model) -> Result<(), AppError> {
    normalize_config_name(&model.name, "LLM配置")?;
    required_config_value(&model.provider, "LLM配置", "provider")?;
    normalize_llm_base_url(&model.base_url, model.api_key_encrypted.as_deref())?;
    required_config_value(&model.model, "LLM配置", "model")?;
    positive_i32(model.max_concurrent, "LLM配置 max_concurrent")?;
    positive_i32(model.timeout_secs, "LLM配置 timeout_secs")?;
    positive_i32(model.max_context_tokens, "LLM配置 max_context_tokens")?;
    positive_i32(model.max_output_tokens, "LLM配置 max_output_tokens")?;
    ensure_output_tokens_below_context(model.max_output_tokens, model.max_context_tokens)?;
    normalize_llm_request_options_json(model.request_options_json.clone())?;
    Ok(())
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
