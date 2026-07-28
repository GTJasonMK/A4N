use a4n_core::error::AppError;
use serde_json::Value;

pub(super) async fn parse_gemini_veo_json_response(
    resp: reqwest::Response,
    label: &str,
) -> Result<Value, AppError> {
    parse_gemini_veo_json_response_inner(resp, label, false)
        .await
        .map(|value| value.expect("not-found responses are disabled"))
}

pub(super) async fn parse_gemini_veo_json_response_allow_not_found(
    resp: reqwest::Response,
    label: &str,
) -> Result<Option<Value>, AppError> {
    parse_gemini_veo_json_response_inner(resp, label, true).await
}

async fn parse_gemini_veo_json_response_inner(
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

pub(super) fn required_operation_name(v: &Value) -> Result<&str, AppError> {
    v.get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Gemini Veo 视频生成响应缺少 operation name"
            ))
        })
}

pub(super) fn operation_error_message(v: &Value) -> Option<String> {
    let message = v
        .pointer("/error/message")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(message.chars().take(500).collect())
}

pub(super) fn operation_is_cancelled(v: &Value) -> bool {
    let code = v
        .pointer("/error/code")
        .and_then(Value::as_str)
        .map(str::to_ascii_uppercase);
    let status = v
        .pointer("/error/status")
        .and_then(Value::as_str)
        .map(str::to_ascii_uppercase);
    code.as_deref()
        .is_some_and(|value| value == "CANCELLED" || value == "CANCELED")
        || status
            .as_deref()
            .is_some_and(|value| value == "CANCELLED" || value == "CANCELED")
}

pub(super) fn find_video_uri(v: &Value) -> Option<&str> {
    for pointer in [
        "/response/generatedVideos/0/video/uri",
        "/response/generatedVideos/0/video/url",
        "/response/generateVideoResponse/generatedVideos/0/video/uri",
        "/response/generateVideoResponse/generatedVideos/0/video/url",
        "/response/generateVideoResponse/generatedSamples/0/video/uri",
        "/response/generateVideoResponse/generatedSamples/0/video/url",
        "/response/videos/0/uri",
        "/response/videos/0/url",
        "/response/video/uri",
        "/response/video/url",
    ] {
        if let Some(uri) = v
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(uri);
        }
    }
    None
}

pub(super) fn find_video_b64(v: &Value) -> Result<Option<&str>, AppError> {
    for pointer in [
        "/response/generatedVideos/0/video/bytesBase64Encoded",
        "/response/generatedVideos/0/video/b64Json",
        "/response/generateVideoResponse/generatedVideos/0/video/bytesBase64Encoded",
        "/response/generateVideoResponse/generatedSamples/0/video/bytesBase64Encoded",
        "/response/videos/0/bytesBase64Encoded",
        "/response/bytesBase64Encoded",
    ] {
        if let Some(value) = v.pointer(pointer) {
            return value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(Some)
                .ok_or_else(|| {
                    AppError::Internal(anyhow::anyhow!(
                        "Gemini Veo 响应字段 {pointer} 必须是非空字符串"
                    ))
                });
        }
    }
    Ok(None)
}

pub(super) fn aspect_ratio_from_size(width: i32, height: i32) -> Option<&'static str> {
    if width <= 0 || height <= 0 {
        return None;
    }
    if width > height {
        Some("16:9")
    } else if height > width {
        Some("9:16")
    } else {
        Some("1:1")
    }
}

pub(super) fn veo_resolution_from_size(width: i32, height: i32) -> &'static str {
    let max_dim = width.max(height);
    if max_dim >= 3840 {
        "4k"
    } else if max_dim >= 1920 {
        "1080p"
    } else {
        "720p"
    }
}
