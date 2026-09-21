use reqwest::header::CONTENT_TYPE;

use a4n_domain::ProviderError as AppError;

use super::format::{
    normalize_video_format_str, video_format_from_content_type_or_bytes, video_format_from_url,
};
use super::transport::{self, ApiKeyAuth};

pub(crate) async fn download_video_url(
    client: &reqwest::Client,
    url: &str,
    hint_format: Option<&str>,
) -> Result<(Vec<u8>, String), AppError> {
    download_video_request(client.get(url), url, hint_format).await
}

pub(crate) async fn download_video_url_with_api_key(
    client: &reqwest::Client,
    url: &str,
    hint_format: Option<&str>,
    api_key: &str,
) -> Result<(Vec<u8>, String), AppError> {
    let request = transport::apply_api_key(
        client.get(url),
        api_key,
        ApiKeyAuth::TrimmedHeader("x-goog-api-key"),
    );
    download_video_request(request, url, hint_format).await
}

async fn download_video_request(
    request: reqwest::RequestBuilder,
    url: &str,
    hint_format: Option<&str>,
) -> Result<(Vec<u8>, String), AppError> {
    let resp = request
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("下载视频失败：{e}")))?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp
            .text()
            .await
            .map_err(|e| AppError::Internal(format!("读取视频下载错误响应失败：{e}")))?;
        return Err(AppError::Internal(format!(
            "下载视频返回 {status}：{}",
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
        .map_err(|e| AppError::Internal(format!("读取视频字节失败：{e}")))?
        .to_vec();
    let format = match hint_format {
        Some(format) => normalize_video_format_str(format)?,
        None => video_format_from_url(url)
            .or_else(|| video_format_from_content_type_or_bytes(content_type.as_deref(), &bytes))
            .ok_or_else(|| {
                AppError::Internal(format!(
                    "无法从 URL、Content-Type 或字节推断视频格式：{url}"
                ))
            })?,
    };

    Ok((bytes, format))
}
