use serde_json::Value;

use a4n_core::error::AppError;

pub(crate) async fn parse_json_response(
    resp: reqwest::Response,
    label: &str,
) -> Result<Value, AppError> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("读取{label}响应失败：{e}")))?;
    if !status.is_success() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "{label}返回 {status}：{}",
            text.chars().take(500).collect::<String>()
        )));
    }
    serde_json::from_str(&text)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{label}响应解析失败：{e}")))
}
