use base64::Engine as _;
use serde_json::Value;

use super::payload::ChatVideoPayload;
use crate::format::{normalize_video_format_str, video_format_from_url};
use a4n_domain::ProviderError as AppError;

pub(super) fn extract_chat_video_payload(body: &str) -> Result<ChatVideoPayload, AppError> {
    let value: Value = serde_json::from_str(body)
        .map_err(|e| AppError::Internal(format!("chat-completions 视频响应解析失败：{e}")))?;
    let message = value
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|c| c.get("message"))
        .ok_or_else(|| {
            AppError::Internal(format!(
                "chat-completions 视频响应缺少 choices[0].message；响应预览：{}",
                preview_video(body)
            ))
        })?;

    if let Some(payload) = scan_message_video(message)? {
        return Ok(payload);
    }
    if let Some(payload) = scan_message_content_video(message)? {
        return Ok(payload);
    }

    Err(AppError::Internal(format!(
        "chat-completions 视频响应未发现视频：扫描了 message.video / message.videos[]、\
         message.content[].video_url、message.content 字符串。响应预览：{}",
        preview_video(body)
    )))
}

/// 第一层：`message.video` 对象或 `message.videos[*]` 数组。
fn scan_message_video(message: &Value) -> Result<Option<ChatVideoPayload>, AppError> {
    if let Some(item) = message.get("video")
        && item.is_object()
        && let Some(payload) = read_video_object(item)?
    {
        return Ok(Some(payload));
    }
    if let Some(arr) = message.get("videos").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(payload) = read_video_object(item)? {
                return Ok(Some(payload));
            }
        }
    }
    Ok(None)
}

/// 第二层：`message.content` —— 数组或字符串。
fn scan_message_content_video(message: &Value) -> Result<Option<ChatVideoPayload>, AppError> {
    let Some(content) = message.get("content") else {
        return Ok(None);
    };
    if let Some(arr) = content.as_array() {
        for part in arr {
            let kind = part.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if !matches!(kind, "video_url" | "video" | "output_video" | "input_video") {
                continue;
            }
            if let Some(s) = part.get("video_url").and_then(|v| v.as_str()) {
                return classify_video_string(s, None).map(Some);
            }
            if let Some(obj) = part.get("video_url").filter(|v| v.is_object())
                && let Some(payload) = read_video_object(obj)?
            {
                return Ok(Some(payload));
            }
            if let Some(payload) = read_video_object(part)? {
                return Ok(Some(payload));
            }
        }
        return Ok(None);
    }
    if let Some(text) = content.as_str() {
        return extract_video_from_text(text);
    }
    Ok(None)
}

fn preview_video(body: &str) -> String {
    let trimmed = body.trim();
    let cap: String = trimmed.chars().take(300).collect();
    if trimmed.chars().count() > 300 {
        format!("{cap}…（已截断）")
    } else {
        cap
    }
}

/// 从一个"视频对象"中读取 url / b64_json / content_type / format。
///
/// 接受形如 `{"url": "...", "format": "mp4"}` 或 `{"b64_json": "...", "content_type": "video/mp4"}`。
fn read_video_object(item: &Value) -> Result<Option<ChatVideoPayload>, AppError> {
    let hint_format = video_format_hint(item)?;
    if let Some(raw) = item
        .get("url")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return classify_video_string(raw, hint_format).map(Some);
    }
    if let Some(raw) = item
        .get("b64_json")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(raw)
            .map_err(|e| AppError::Internal(format!("视频 base64 解码失败：{e}")))?;
        return Ok(Some(ChatVideoPayload::Base64 {
            format: hint_format,
            bytes,
        }));
    }
    Ok(None)
}

fn video_format_hint(item: &Value) -> Result<Option<String>, AppError> {
    if let Some(raw) = item
        .get("format")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Ok(Some(normalize_video_format_str(raw)?));
    }
    if let Some(mime) = item
        .get("content_type")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let ext = mime.strip_prefix("video/").ok_or_else(|| {
            AppError::Internal(format!(
                "chat-completions 视频 content_type 非 video/* ：{mime}"
            ))
        })?;
        return Ok(Some(normalize_video_format_str(ext)?));
    }
    Ok(None)
}

fn extract_video_from_text(text: &str) -> Result<Option<ChatVideoPayload>, AppError> {
    if let Some(start) = text.find("data:video/") {
        let rest = &text[start..];
        let end = rest
            .find(|c: char| c.is_whitespace() || c == ')' || c == '"' || c == '\'')
            .unwrap_or(rest.len());
        let candidate = &rest[..end];
        return classify_video_string(candidate, None).map(Some);
    }
    if let Some(url) = find_markdown_video(text) {
        return classify_video_string(url, None).map(Some);
    }
    if let Some(url) = find_bare_video_url(text) {
        return classify_video_string(url, None).map(Some);
    }
    Ok(None)
}

/// 解析形如 `![alt](https://example.com/x.mp4)` 的 markdown 链接，返回 URL 部分。
fn find_markdown_video(text: &str) -> Option<&str> {
    let bang = text.find("![")?;
    let after_alt_open = bang + 2;
    let alt_close_rel = text[after_alt_open..].find(']')?;
    let alt_close = after_alt_open + alt_close_rel;
    if text.as_bytes().get(alt_close + 1).copied() != Some(b'(') {
        return None;
    }
    let url_start = alt_close + 2;
    let url_end_rel = text[url_start..].find(')')?;
    let url = text[url_start..url_start + url_end_rel].trim();
    let url = url.split_whitespace().next().unwrap_or(url);
    if url.is_empty() { None } else { Some(url) }
}

/// 从纯文本中找一个以视频扩展名结尾的 http(s) URL（首个匹配即可）。
fn find_bare_video_url(text: &str) -> Option<&str> {
    for scheme in ["https://", "http://"] {
        if let Some(start) = text.find(scheme) {
            let rest = &text[start..];
            let end = rest
                .find(|c: char| {
                    c.is_whitespace() || c == ')' || c == '"' || c == '\'' || c == '<' || c == '>'
                })
                .unwrap_or(rest.len());
            let candidate = &rest[..end];
            if video_format_from_url(candidate).is_some() {
                return Some(candidate);
            }
        }
    }
    None
}

fn classify_video_string(
    raw: &str,
    hint_format: Option<String>,
) -> Result<ChatVideoPayload, AppError> {
    let raw = raw.trim();
    if let Some(rest) = raw.strip_prefix("data:") {
        let (header, payload) = rest.split_once(',').ok_or_else(|| {
            AppError::Internal("chat-completions video data URI 缺少 ','".to_string())
        })?;
        let payload = payload.trim();
        let mime = header.split(';').next().unwrap_or("").trim();
        let ext = mime.strip_prefix("video/").ok_or_else(|| {
            AppError::Internal(format!("chat-completions 不支持的 data URI mime：{mime}"))
        })?;
        let format = normalize_video_format_str(ext)?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .map_err(|e| AppError::Internal(format!("视频 base64 解码失败：{e}")))?;
        return Ok(ChatVideoPayload::DataUri { format, bytes });
    }
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return Ok(ChatVideoPayload::Url {
            url: raw.to_string(),
            hint_format: hint_format.or_else(|| video_format_from_url(raw)),
        });
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|e| AppError::Internal(format!("视频 base64 解码失败：{e}")))?;
    Ok(ChatVideoPayload::Base64 {
        format: hint_format,
        bytes,
    })
}
