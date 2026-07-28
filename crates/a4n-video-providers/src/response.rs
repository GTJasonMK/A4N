use serde_json::Value;

use a4n_core::error::AppError;

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
        .map_err(|e| AppError::Internal(anyhow::anyhow!("读取{label}响应失败：{e}")))?;
    if allow_not_found && status == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "{label}返回 {status}：{}",
            text.chars().take(500).collect::<String>()
        )));
    }
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{label}响应解析失败：{e}")))
}
