use a4n_domain::ProviderError as AppError;
use serde_json::Value;

pub(super) fn error_body_preview(value: &str) -> String {
    value.chars().take(500).collect::<String>()
}

pub(super) fn completed_video_url(v: &Value) -> Option<&str> {
    for pointer in [
        "/video/url",
        "/video_url",
        "/download_url",
        "/output_url",
        "/url",
    ] {
        if let Some(value) = v.pointer(pointer).and_then(Value::as_str).map(str::trim)
            && !value.is_empty()
        {
            return Some(value);
        }
    }
    None
}

pub(super) fn video_id(v: &Value) -> Result<String, AppError> {
    for pointer in ["/request_id", "/id", "/video_id", "/provider_job_id"] {
        if let Some(value) = v.pointer(pointer).and_then(Value::as_str).map(str::trim)
            && !value.is_empty()
        {
            return Ok(value.to_string());
        }
    }
    Err(AppError::Internal(
        "CLIProxy 视频生成响应缺少 id".to_string(),
    ))
}

pub(super) fn status(v: &Value) -> Option<String> {
    v.pointer("/status")
        .or_else(|| v.pointer("/state"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
}

pub(super) fn required_status(v: &Value) -> Result<String, AppError> {
    status(v).ok_or_else(|| AppError::Internal("CLIProxy 视频任务响应缺少 status".to_string()))
}

pub(super) fn is_completed_status(status: Option<&str>) -> bool {
    matches!(
        status,
        Some("completed" | "complete" | "succeeded" | "success" | "done")
    )
}

pub(super) fn failure_message(v: &Value) -> Option<String> {
    let status_failed = matches!(
        status(v).as_deref(),
        Some("failed" | "failure" | "error" | "expired" | "cancelled" | "canceled")
    );
    let message = v
        .pointer("/error/message")
        .and_then(Value::as_str)
        .or_else(|| v.pointer("/error").and_then(Value::as_str))
        .or_else(|| v.pointer("/message").and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if status_failed {
        Some(message.unwrap_or_else(|| "视频生成失败".to_string()))
    } else {
        None
    }
}

pub(super) fn aspect_ratio_from_size(width: i32, height: i32) -> Option<&'static str> {
    if width <= 0 || height <= 0 {
        return None;
    }
    if width == height {
        return Some("1:1");
    }
    if width * 9 == height * 16 {
        return Some("16:9");
    }
    if width * 16 == height * 9 {
        return Some("9:16");
    }
    if width * 3 == height * 4 {
        return Some("4:3");
    }
    if width * 4 == height * 3 {
        return Some("3:4");
    }
    if width * 2 == height * 3 {
        return Some("3:2");
    }
    if width * 3 == height * 2 {
        return Some("2:3");
    }
    None
}

pub(super) fn url_escape_path_segment(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}
