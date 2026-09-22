use serde_json::Value;

use a4n_domain::ProviderError as AppError;

pub(crate) async fn parse_json_response(
    resp: reqwest::Response,
    label: &str,
) -> Result<Value, AppError> {
    parse_json_response_inner(resp, label, false)
        .await
        .map(|value| value.expect("not-found responses are disabled"))
}

/// Parse a provider JSON response while preserving a remote 404 as `None`.
/// Cleanup status checks use this path so a missing remote job is idempotent.
pub(crate) async fn parse_json_response_allow_not_found(
    resp: reqwest::Response,
    label: &str,
) -> Result<Option<Value>, AppError> {
    parse_json_response_inner(resp, label, true).await
}

async fn parse_json_response_inner(
    resp: reqwest::Response,
    label: &str,
    allow_not_found: bool,
) -> Result<Option<Value>, AppError> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| AppError::Internal(format!("读取{label}响应失败：{e}")))?;
    if allow_not_found && status == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(non_success_error(status, label, &text));
    }
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|e| AppError::Internal(format!("{label}响应解析失败：{e}")))
}

fn non_success_error(status: reqwest::StatusCode, label: &str, text: &str) -> AppError {
    let detail = text.chars().take(500).collect::<String>();
    let message = if status == reqwest::StatusCode::PAYLOAD_TOO_LARGE {
        format!(
            "{label}返回 413 Payload Too Large：请求体超过 provider 限制；请使用 public_url 或 remote_temp_upload，避免 inline_data_url。响应：{detail}"
        )
    } else {
        format!("{label}返回 {status}：{detail}")
    };
    if status.is_client_error() {
        AppError::InvalidArgument(message)
    } else {
        AppError::Internal(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_too_large_is_a_non_retryable_provider_input_error() {
        let error = non_success_error(
            reqwest::StatusCode::PAYLOAD_TOO_LARGE,
            "视频生成",
            "gateway limit",
        );
        assert!(
            matches!(error, AppError::InvalidArgument(message) if message.contains("413 Payload Too Large") && message.contains("remote_temp_upload"))
        );
    }

    #[test]
    fn server_provider_errors_remain_internal() {
        let error = non_success_error(
            reqwest::StatusCode::BAD_GATEWAY,
            "视频生成",
            "upstream unavailable",
        );
        assert!(
            matches!(error, AppError::Internal(message) if message.to_string().contains("502"))
        );
    }
}
