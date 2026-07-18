use super::super::super::types::{AllConfigsExport, LlmRow};
use super::super::shared::{positive_i32, required_text, validate_default_named_rows};
use a4n_core::error::AppError;
use a4n_llm::provider::normalize_request_options_json;

pub(super) fn validate_llm_rows(incoming: &AllConfigsExport) -> Result<(), String> {
    validate_default_named_rows(
        "LLM 配置",
        &incoming.llm_configs,
        |row| row.is_default,
        |row| &row.name,
        |label, row| {
            required_text(&row.provider, label, "provider")?;
            required_text(&row.base_url, label, "base_url")?;
            required_text(&row.model, label, "model")?;
            positive_i32(row.max_concurrent, label, "max_concurrent")?;
            positive_i32(row.timeout_secs, label, "timeout_secs")?;
            positive_i32(row.max_context_tokens, label, "max_context_tokens")?;
            positive_i32(row.max_output_tokens, label, "max_output_tokens")?;
            if row.max_output_tokens >= row.max_context_tokens {
                return Err(format!(
                    "{label} max_output_tokens 必须小于 max_context_tokens"
                ));
            }
            validate_llm_request_options(row, label)
        },
    )
}

fn validate_llm_request_options(row: &LlmRow, label: &str) -> Result<(), String> {
    normalize_request_options_json(row.request_options_json.clone())
        .map(|_| ())
        .map_err(|err| format!("{label} request_options_json 无效：{}", plain_error(err)))
}

pub(super) fn plain_error(err: AppError) -> String {
    match err {
        AppError::InvalidArgument(message)
        | AppError::ConfigError(message)
        | AppError::LlmError(message)
        | AppError::Timeout(message) => message,
        other => other.to_string(),
    }
}
