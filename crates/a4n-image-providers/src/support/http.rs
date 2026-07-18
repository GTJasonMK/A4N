use super::errors::{error_chain, internal_error, request_error};
use super::format::image_format_hint;
use a4n_core::error::AppError;
use reqwest::header::CONTENT_TYPE;
use std::time::Duration;

const IMAGE_CONNECT_TIMEOUT_SECS: u64 = 20;
const IMAGE_READ_TIMEOUT_SECS: u64 = 300;
pub(crate) fn build_image_http_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(IMAGE_CONNECT_TIMEOUT_SECS))
        .read_timeout(Duration::from_secs(IMAGE_READ_TIMEOUT_SECS))
        .pool_idle_timeout(Duration::from_secs(30))
        .http1_only()
        .build()
        .map_err(|e| internal_error(format!("图片 HTTP 客户端初始化失败：{e}")))
}

pub(crate) async fn send_image_request(
    label: &str,
    request: reqwest::RequestBuilder,
) -> Result<reqwest::Response, AppError> {
    request.send().await.map_err(|e| request_error(label, e))
}
pub(crate) async fn response_text(
    resp: reqwest::Response,
    label: &str,
) -> Result<String, AppError> {
    resp.text()
        .await
        .map_err(|e| internal_error(format!("{label}响应读取失败：{}", error_chain(&e))))
}

pub(crate) async fn response_bytes(
    resp: reqwest::Response,
    label: &str,
) -> Result<Vec<u8>, AppError> {
    let status = resp.status();
    if !status.is_success() {
        let text = response_text(resp, label).await?;
        return Err(internal_error(format!(
            "{label}返回 {status}：{}",
            text.chars().take(500).collect::<String>()
        )));
    }
    resp.bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|e| internal_error(format!("{label}字节读取失败：{}", error_chain(&e))))
}

pub(crate) async fn download_image_url(
    client: &reqwest::Client,
    url: &str,
    declared_format: Option<&str>,
) -> Result<(Vec<u8>, String), AppError> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| request_error("下载图像请求失败", e))?;
    let status = resp.status();
    if !status.is_success() {
        let text = response_text(resp, "下载图像").await?;
        return Err(internal_error(format!(
            "下载图像返回 {status}：{}",
            text.chars().take(500).collect::<String>()
        )));
    }

    let content_type = resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let bytes = resp
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|e| internal_error(format!("下载图像字节读取失败：{}", error_chain(&e))))?;
    let format = image_format_hint(declared_format, Some(url), content_type.as_deref(), &bytes)?;
    Ok((bytes, format))
}
