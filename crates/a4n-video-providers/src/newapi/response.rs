use a4n_core::error::AppError;
use serde_json::Value;

pub(super) struct VideoUrlPayload<'a> {
    pub(super) url: &'a str,
    pub(super) format: Option<&'a str>,
}

pub(super) struct VideoB64Payload<'a> {
    pub(super) b64: &'a str,
    pub(super) format: Option<&'a str>,
}

pub(super) fn extract_task_id(v: &Value) -> Result<Option<String>, AppError> {
    for pointer in ["/task_id", "/id", "/data/task_id", "/data/id"] {
        if let Some(text) = optional_pointer_str(v, pointer)? {
            return Ok(Some(text.to_string()));
        }
    }
    Ok(None)
}

pub(super) fn extract_status(v: &Value) -> Option<&str> {
    [
        "/data/data/status",
        "/data/status",
        "/status",
        "/state",
        "/data/state",
    ]
    .into_iter()
    .find_map(|pointer| v.pointer(pointer).and_then(Value::as_str).map(str::trim))
    .filter(|text| !text.is_empty())
}

pub(super) fn find_video_url(v: &Value) -> Result<Option<VideoUrlPayload<'_>>, AppError> {
    for (url_pointer, format_pointers) in [
        (
            "/data/data/video/url",
            &["/data/data/video/format", "/data/data/format"][..],
        ),
        ("/data/data/url", &["/data/data/format"][..]),
        (
            "/data/video/url",
            &["/data/video/format", "/data/format"][..],
        ),
        ("/data/url", &["/data/format"][..]),
        ("/video/url", &["/video/format", "/format"][..]),
        ("/url", &["/format"][..]),
    ] {
        let Some(url) = optional_pointer_str(v, url_pointer)? else {
            continue;
        };
        if !looks_like_http_url(url) {
            continue;
        }
        let format = first_optional_pointer_str(v, format_pointers)?;
        return Ok(Some(VideoUrlPayload { url, format }));
    }
    Ok(None)
}

pub(super) fn find_video_b64(v: &Value) -> Result<Option<VideoB64Payload<'_>>, AppError> {
    for (b64_pointer, format_pointers) in [
        (
            "/data/data/video/b64_json",
            &["/data/data/video/format", "/data/data/format"][..],
        ),
        ("/data/data/b64_json", &["/data/data/format"][..]),
        (
            "/data/video/b64_json",
            &["/data/video/format", "/data/format"][..],
        ),
        ("/data/b64_json", &["/data/format"][..]),
        ("/video/b64_json", &["/video/format", "/format"][..]),
        ("/b64_json", &["/format"][..]),
    ] {
        let Some(b64) = optional_pointer_str(v, b64_pointer)? else {
            continue;
        };
        let format = first_optional_pointer_str(v, format_pointers)?;
        return Ok(Some(VideoB64Payload { b64, format }));
    }
    Ok(None)
}

pub(super) fn optional_pointer_str<'a>(
    v: &'a Value,
    pointer: &str,
) -> Result<Option<&'a str>, AppError> {
    let Some(value) = v.pointer(pointer) else {
        return Ok(None);
    };
    let text = value.as_str().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "New API 视频响应字段 {pointer} 必须是字符串"
        ))
    })?;
    let text = text.trim();
    if text.is_empty() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "New API 视频响应字段 {pointer} 必须是非空字符串"
        )));
    }
    Ok(Some(text))
}

pub(super) fn first_optional_pointer_str<'a>(
    v: &'a Value,
    pointers: &[&str],
) -> Result<Option<&'a str>, AppError> {
    for pointer in pointers {
        if let Some(text) = optional_pointer_str(v, pointer)? {
            return Ok(Some(text));
        }
    }
    Ok(None)
}

pub(super) fn looks_like_http_url(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://")
}

pub(super) fn api_error_suffix(v: &Value) -> String {
    v.pointer("/error/message")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| format!("；错误：{}", text.chars().take(500).collect::<String>()))
        .unwrap_or_default()
}

pub(super) fn extract_failure_message(v: &Value) -> String {
    for pointer in [
        "/data/fail_reason",
        "/fail_reason",
        "/data/error/message",
        "/error/message",
        "/data/result_url",
        "/message",
    ] {
        if let Some(text) = v
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
        {
            return text.chars().take(500).collect();
        }
    }
    "New API 视频任务失败，后端未返回错误详情".into()
}
