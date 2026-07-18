use crate::support::{internal_error, normalize_image_format};
use a4n_core::error::AppError;
use base64::Engine as _;

/// chat-completions 响应中提取出的图像负载。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ChatImagePayload {
    /// `data:image/<ext>;base64,XXX`，扩展名已知。
    DataUri { format: String, bytes: Vec<u8> },
    /// 纯 base64 字符串，未带 data URI 头，格式需由字节魔数判断。
    Base64 {
        format: Option<String>,
        bytes: Vec<u8>,
    },
    /// HTTP(S) URL，需要后续下载。
    Url(String),
}

pub(super) fn extract_chat_image_payload(body: &str) -> Result<ChatImagePayload, AppError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| internal_error(format!("chat-completions 响应解析失败：{e}")))?;
    if let Some(message) = first_message(&value) {
        if let Some(payload) = scan_message_images(message)? {
            return Ok(payload);
        }
        if let Some(payload) = scan_message_content(message)? {
            return Ok(payload);
        }
    }
    if let Some(payload) = scan_any_image_payload(&value)? {
        return Ok(payload);
    }

    Err(internal_error(format!(
        "chat-completions 响应未发现图片：扫描了 message.images[]、message.content[].image_url、\
         message.content 字符串，以及整个响应中的图片语义字段。响应预览：{}",
        preview(body)
    )))
}

fn first_message(value: &serde_json::Value) -> Option<&serde_json::Value> {
    value
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|c| c.get("message"))
}

/// 第一层：`message.images[*]` 中的常见字段（OpenRouter Gemini-image 等）。
fn scan_message_images(message: &serde_json::Value) -> Result<Option<ChatImagePayload>, AppError> {
    let Some(arr) = message.get("images").and_then(|v| v.as_array()) else {
        return Ok(None);
    };
    for item in arr {
        if let Some(payload) = read_image_part_url(item)? {
            return Ok(Some(payload));
        }
    }
    Ok(None)
}

/// 第二层：`message.content` —— 数组（找 type=image_url 项）或字符串（扫 data URI / markdown）。
fn scan_message_content(message: &serde_json::Value) -> Result<Option<ChatImagePayload>, AppError> {
    let Some(content) = message.get("content") else {
        return Ok(None);
    };
    if let Some(arr) = content.as_array() {
        for part in arr {
            let kind = part.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if matches!(kind, "image_url" | "image" | "output_image" | "input_image")
                && let Some(payload) = read_image_part_url(part)?
            {
                return Ok(Some(payload));
            }
        }
        return Ok(None);
    }
    if let Some(text) = content.as_str() {
        return extract_image_from_text(text);
    }
    Ok(None)
}

fn scan_any_image_payload(value: &serde_json::Value) -> Result<Option<ChatImagePayload>, AppError> {
    scan_any_image_payload_at(value, None)
}

fn scan_any_image_payload_at(
    value: &serde_json::Value,
    key_hint: Option<&str>,
) -> Result<Option<ChatImagePayload>, AppError> {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(payload) = read_image_part_url(value)? {
                return Ok(Some(payload));
            }
            for (key, child) in map {
                if let Some(payload) = scan_any_image_payload_at(child, Some(key))? {
                    return Ok(Some(payload));
                }
            }
            Ok(None)
        }
        serde_json::Value::Array(items) => {
            for item in items {
                if let Some(payload) = scan_any_image_payload_at(item, key_hint)? {
                    return Ok(Some(payload));
                }
            }
            Ok(None)
        }
        serde_json::Value::String(text) => {
            if image_payload_key_hint(key_hint) {
                return classify_image_string(text).map(Some);
            }
            extract_image_from_text(text)
        }
        _ => Ok(None),
    }
}

fn image_payload_key_hint(key: Option<&str>) -> bool {
    let Some(key) = key else {
        return false;
    };
    let key = key.to_ascii_lowercase();
    key == "url"
        || key == "image_url"
        || key == "b64_json"
        || key == "base64"
        || key == "data_uri"
        || key.ends_with("_url")
        || key.contains("image")
}

fn preview(body: &str) -> String {
    let trimmed = body.trim();
    let cap: String = trimmed.chars().take(300).collect();
    if trimmed.chars().count() > 300 {
        format!("{cap}…（已截断）")
    } else {
        cap
    }
}

fn read_image_part_url(item: &serde_json::Value) -> Result<Option<ChatImagePayload>, AppError> {
    let raw_url = item
        .get("image_url")
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                Some(s)
            } else {
                v.get("url").and_then(|u| u.as_str())
            }
        })
        .or_else(|| item.get("url").and_then(|v| v.as_str()))
        .or_else(|| item.get("b64_json").and_then(|v| v.as_str()))
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let raw_url = match raw_url {
        Some(v) => v,
        None => return Ok(None),
    };
    classify_image_string(raw_url).map(Some)
}

fn extract_image_from_text(text: &str) -> Result<Option<ChatImagePayload>, AppError> {
    if let Some(start) = text.find("data:image/") {
        let rest = &text[start..];
        let end = rest
            .find(|c: char| c.is_whitespace() || c == ')' || c == '"' || c == '\'')
            .unwrap_or(rest.len());
        let candidate = &rest[..end];
        return classify_image_string(candidate).map(Some);
    }
    if let Some(url) = find_markdown_image(text) {
        return classify_image_string(url).map(Some);
    }
    Ok(None)
}

/// 解析形如 `![alt](https://example.com/x.png)` 的 markdown 图片，返回 URL 部分。
fn find_markdown_image(text: &str) -> Option<&str> {
    let bang = text.find("![")?;
    let after_alt_open = bang + 2;
    let alt_close_rel = text[after_alt_open..].find(']')?;
    let alt_close = after_alt_open + alt_close_rel;
    // 紧跟 `](`
    if text.as_bytes().get(alt_close + 1).copied() != Some(b'(') {
        return None;
    }
    let url_start = alt_close + 2;
    let url_end_rel = text[url_start..].find(')')?;
    let url = text[url_start..url_start + url_end_rel].trim();
    let url = url.split_whitespace().next().unwrap_or(url);
    if url.is_empty() { None } else { Some(url) }
}

fn classify_image_string(raw: &str) -> Result<ChatImagePayload, AppError> {
    let raw = raw.trim();
    if let Some(rest) = raw.strip_prefix("data:") {
        let (header, payload) = rest
            .split_once(',')
            .ok_or_else(|| internal_error("chat-completions data URI 缺少 ','"))?;
        let payload = payload.trim();
        let mime = header.split(';').next().unwrap_or("").trim();
        let format = mime
            .strip_prefix("image/")
            .map(normalize_image_format)
            .transpose()?
            .ok_or_else(|| {
                internal_error(format!("chat-completions 不支持的 data URI mime：{mime}"))
            })?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .map_err(|e| internal_error(format!("base64 解码失败：{e}")))?;
        return Ok(ChatImagePayload::DataUri { format, bytes });
    }
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return Ok(ChatImagePayload::Url(raw.to_string()));
    }
    // 纯 base64 串：先解码，再用魔数嗅探。
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|e| internal_error(format!("base64 解码失败：{e}")))?;
    Ok(ChatImagePayload::Base64 {
        format: None,
        bytes,
    })
}
