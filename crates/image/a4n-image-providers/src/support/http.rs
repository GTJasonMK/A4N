use super::errors::{error_chain, internal_error, request_error};
use super::format::image_format_hint;
use a4n_domain::ProviderError as AppError;
use std::time::Duration;

const IMAGE_CONNECT_TIMEOUT_SECS: u64 = 20;
const IMAGE_READ_TIMEOUT_SECS: u64 = 300;
const IMAGE_DOWNLOAD_MAX_ATTEMPTS: usize = 3;
const IMAGE_DOWNLOAD_RETRY_DELAY_MS: u64 = 250;

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
    let mut last_error = None;
    for attempt in 1..=IMAGE_DOWNLOAD_MAX_ATTEMPTS {
        match download_image_url_once(client, url, declared_format).await {
            Ok(result) => return Ok(result),
            Err(error) if attempt < IMAGE_DOWNLOAD_MAX_ATTEMPTS && error.is_retryable() => {
                tracing::warn!(
                    attempt,
                    max_attempts = IMAGE_DOWNLOAD_MAX_ATTEMPTS,
                    "下载图像遇到瞬时网络错误，准备重试"
                );
                last_error = Some(error);
                tokio::time::sleep(Duration::from_millis(
                    IMAGE_DOWNLOAD_RETRY_DELAY_MS * attempt as u64,
                ))
                .await;
            }
            Err(error) => return Err(error.into_app_error()),
        }
    }

    Err(last_error
        .expect("image download retry loop must retain its last error")
        .into_app_error())
}

async fn download_image_url_once(
    client: &reqwest::Client,
    url: &str,
    declared_format: Option<&str>,
) -> Result<(Vec<u8>, String), ImageDownloadError> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(ImageDownloadError::Request)?;
    let status = resp.status();
    if !status.is_success() {
        let text = response_text(resp, "下载图像")
            .await
            .map_err(ImageDownloadError::Other)?;
        return Err(ImageDownloadError::Other(internal_error(format!(
            "下载图像返回 {status}：{}",
            text.chars().take(500).collect::<String>()
        ))));
    }

    let bytes = resp
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(ImageDownloadError::Body)?;
    let format = image_format_hint(declared_format, &bytes).map_err(ImageDownloadError::Other)?;
    Ok((bytes, format))
}

enum ImageDownloadError {
    Request(reqwest::Error),
    Body(reqwest::Error),
    Other(AppError),
}

impl ImageDownloadError {
    fn is_retryable(&self) -> bool {
        match self {
            Self::Request(error) | Self::Body(error) => {
                error.is_connect()
                    || error.is_timeout()
                    || error.is_request()
                    || error.is_body()
                    || error.is_decode()
            }
            Self::Other(_) => false,
        }
    }

    fn into_app_error(self) -> AppError {
        match self {
            Self::Request(error) => request_error("下载图像请求失败", error),
            Self::Body(error) => {
                internal_error(format!("下载图像字节读取失败：{}", error_chain(&error)))
            }
            Self::Other(error) => error,
        }
    }
}
