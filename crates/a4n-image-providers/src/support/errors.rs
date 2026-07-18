use a4n_core::error::AppError;
use reqwest::StatusCode;
use std::error::Error;

const HTTP_ERROR_PREVIEW_CHARS: usize = 500;
pub(crate) fn internal_error(message: impl Into<String>) -> AppError {
    AppError::Internal(anyhow::anyhow!(message.into()))
}

pub(crate) fn request_error(label: &str, error: reqwest::Error) -> AppError {
    internal_error(format!("{label}：{}", error_chain(&error)))
}

pub(crate) fn image_provider_http_error(label: &str, status: StatusCode, body: &str) -> AppError {
    let details = parse_provider_error(body);
    let message = details
        .message
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::trim)
        .map(str::to_string)
        .unwrap_or_else(|| preview_chars(body, HTTP_ERROR_PREVIEW_CHARS));

    if is_content_policy_rejection(&details, body) {
        return AppError::InvalidArgument(format!(
            "{label}被内容安全策略拒绝：{message}。请修改提示词中被供应商拒绝的高风险描写后重试"
        ));
    }

    if status == StatusCode::BAD_REQUEST
        && details
            .error_type
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case("invalid_request_error"))
    {
        return AppError::InvalidArgument(format!("{label}请求被供应商拒绝：{message}"));
    }

    internal_error(format!("{label}返回 {status}：{message}"))
}
pub(super) fn error_chain(error: &dyn Error) -> String {
    let mut parts = vec![error.to_string()];
    let mut source = error.source();
    while let Some(err) = source {
        let text = err.to_string();
        if !parts.iter().any(|part| part == &text) {
            parts.push(text);
        }
        source = err.source();
    }
    parts.join("；原因：")
}

#[derive(Debug, Default)]
struct ProviderErrorDetails {
    message: Option<String>,
    error_type: Option<String>,
    code: Option<String>,
}

fn parse_provider_error(body: &str) -> ProviderErrorDetails {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return ProviderErrorDetails::default();
    };
    let error = value.get("error").unwrap_or(&value);
    ProviderErrorDetails {
        message: string_field(error, "message"),
        error_type: string_field(error, "type"),
        code: string_field(error, "code"),
    }
}

fn string_field(value: &serde_json::Value, field: &str) -> Option<String> {
    let raw = value.get(field)?;
    if let Some(text) = raw.as_str() {
        return non_empty_string(text);
    }
    if raw.is_null() {
        None
    } else {
        non_empty_string(&raw.to_string())
    }
}

fn non_empty_string(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn is_content_policy_rejection(details: &ProviderErrorDetails, body: &str) -> bool {
    details
        .code
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("content_policy_violation"))
        || contains_policy_rejection_hint(body)
}

fn contains_policy_rejection_hint(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.contains("content_policy_violation")
        || (lower.contains("content policy") && lower.contains("violation"))
        || (lower.contains("safety") && (lower.contains("policy") || lower.contains("refus")))
        || body.contains("内容安全")
        || body.contains("安全策略")
        || body.contains("不能根据这个描述生成图像")
}

fn preview_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}
