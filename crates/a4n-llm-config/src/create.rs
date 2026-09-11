use a4n_domain::config::{normalize_config_name, positive_i32, required_config_value};
use a4n_error::AppError;
use a4n_llm::provider::LlmProtocolKind;

use super::{
    contracts::{CreateLlmConfigInput, NormalizedLlmConfig},
    limits::ensure_output_tokens_below_context,
    validation::{normalize_llm_base_url, normalize_llm_extra_body_json},
};

pub fn normalize_create_llm_config_input(
    input: CreateLlmConfigInput<'_>,
) -> Result<NormalizedLlmConfig, AppError> {
    let CreateLlmConfigInput {
        name,
        provider,
        base_url,
        api_key_encrypted,
        model,
        max_concurrent,
        timeout_secs,
        max_context_tokens,
        max_output_tokens,
        extra_body_json,
    } = input;
    let name = normalize_config_name(name, "LLM配置").map_err(AppError::InvalidArgument)?;
    let provider = required_config_value(provider, "LLM配置", "provider")
        .map_err(AppError::InvalidArgument)?;
    if LlmProtocolKind::for_provider(&provider).is_none() {
        return Err(AppError::InvalidArgument(format!(
            "LLM配置 provider 不受支持：{provider}"
        )));
    }
    let base_url = normalize_llm_base_url(base_url, api_key_encrypted)?;
    let model =
        required_config_value(model, "LLM配置", "model").map_err(AppError::InvalidArgument)?;
    let max_concurrent = positive_i32(max_concurrent, "LLM配置 max_concurrent")
        .map_err(AppError::InvalidArgument)?;
    let timeout_secs =
        positive_i32(timeout_secs, "LLM配置 timeout_secs").map_err(AppError::InvalidArgument)?;
    let max_context_tokens = positive_i32(max_context_tokens, "LLM配置 max_context_tokens")
        .map_err(AppError::InvalidArgument)?;
    let max_output_tokens = positive_i32(max_output_tokens, "LLM配置 max_output_tokens")
        .map_err(AppError::InvalidArgument)?;
    ensure_output_tokens_below_context(max_output_tokens, max_context_tokens)?;
    let extra_body_json = normalize_llm_extra_body_json(extra_body_json)?;
    Ok(NormalizedLlmConfig {
        name,
        provider,
        base_url,
        api_key_encrypted: api_key_encrypted.map(str::to_string),
        model,
        max_concurrent,
        timeout_secs,
        max_context_tokens,
        max_output_tokens,
        extra_body_json,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn input<'a>() -> CreateLlmConfigInput<'a> {
        CreateLlmConfigInput {
            name: "  primary  ",
            provider: "  openai  ",
            base_url: "  https://llm.example/v1  ",
            api_key_encrypted: Some("secret"),
            model: "  gpt-test  ",
            max_concurrent: 2,
            timeout_secs: 30,
            max_context_tokens: 4096,
            max_output_tokens: 1024,
            extra_body_json: Some(json!({
                "extra_body": {"reasoning_effort": "low"}
            })),
        }
    }

    #[test]
    fn normalizes_fields_and_extra_body() {
        let normalized = normalize_create_llm_config_input(input()).expect("valid config");

        assert_eq!(normalized.name, "primary");
        assert_eq!(normalized.provider, "openai");
        assert_eq!(normalized.base_url, "https://llm.example/v1");
        assert_eq!(normalized.model, "gpt-test");
        assert_eq!(
            normalized.extra_body_json,
            Some(json!({
                "extra_body": {"reasoning_effort": "low"}
            }))
        );
    }

    #[test]
    fn rejects_output_tokens_not_below_context() {
        let mut input = input();
        input.max_output_tokens = input.max_context_tokens;

        assert_eq!(
            normalize_create_llm_config_input(input)
                .expect_err("invalid token relation")
                .to_string(),
            "参数无效：LLM配置 max_output_tokens 必须小于 max_context_tokens"
        );
    }

    #[test]
    fn rejects_reserved_request_option_fields() {
        let mut input = input();
        input.extra_body_json = Some(json!({"model": "override"}));

        assert!(
            normalize_create_llm_config_input(input)
                .expect_err("reserved request option")
                .to_string()
                .contains("LLM配置 extra_body_json 无效")
        );
    }
}
