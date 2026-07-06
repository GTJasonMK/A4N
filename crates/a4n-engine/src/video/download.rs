use reqwest::header::CONTENT_TYPE;

use a4n_core::error::AppError;

use super::format::{
    detect_video_format_from_bytes, normalize_video_format_str, video_format_from_url,
};

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
    let mut request = client.get(url);
    if !api_key.trim().is_empty() {
        request = request.header("x-goog-api-key", api_key.trim());
    }
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
        .map_err(|e| AppError::Internal(anyhow::anyhow!("下载视频失败：{e}")))?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp
            .text()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("读取视频下载错误响应失败：{e}")))?;
        return Err(AppError::Internal(anyhow::anyhow!(
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
        .map_err(|e| AppError::Internal(anyhow::anyhow!("读取视频字节失败：{e}")))?
        .to_vec();
    let format = match hint_format {
        Some(format) => normalize_video_format_str(format)?,
        None => video_format_from_url(url)
            .or_else(|| content_type.as_deref().and_then(format_from_content_type))
            .or_else(|| detect_video_format_from_bytes(&bytes).ok())
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!(
                    "无法从 URL、Content-Type 或字节推断视频格式：{url}"
                ))
            })?,
    };

    Ok((bytes, format))
}

fn format_from_content_type(content_type: &str) -> Option<String> {
    let media_type = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase();
    match media_type.as_str() {
        "video/mp4" => Some("mp4".into()),
        "video/webm" => Some("webm".into()),
        "video/quicktime" => Some("mov".into()),
        "video/x-m4v" => Some("m4v".into()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_format_from_content_type() {
        assert_eq!(
            format_from_content_type("video/mp4; charset=utf-8"),
            Some("mp4".into())
        );
        assert_eq!(
            format_from_content_type("video/quicktime"),
            Some("mov".into())
        );
        assert_eq!(format_from_content_type("application/json"), None);
    }
}
