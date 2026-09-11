use a4n_domain::ProviderError as AppError;
use serde_json::{Value, json};

pub(super) fn image_inline_data(raw: &str, label: &str) -> Result<Value, AppError> {
    let raw = raw.trim();
    let Some(rest) = raw.strip_prefix("data:") else {
        return Err(AppError::InvalidArgument(format!(
            "Gemini Veo {label} 必须是 data URL"
        )));
    };
    let Some((metadata, data)) = rest.split_once(',') else {
        return Err(AppError::InvalidArgument(format!(
            "Gemini Veo {label} data URL 缺少 base64 数据"
        )));
    };
    let mime_type = metadata
        .split(';')
        .next()
        .map(str::trim)
        .filter(|value| value.starts_with("image/"))
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("Gemini Veo {label} 必须是 image/* data URL"))
        })?;
    let is_base64 = metadata
        .split(';')
        .skip(1)
        .any(|part| part.trim().eq_ignore_ascii_case("base64"));
    if !is_base64 {
        return Err(AppError::InvalidArgument(format!(
            "Gemini Veo {label} data URL 必须使用 base64 编码"
        )));
    }
    let data = data.trim();
    if data.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "Gemini Veo {label} base64 数据不能为空"
        )));
    }
    Ok(json!({
        "inlineData": {
            "mimeType": mime_type,
            "data": data,
        }
    }))
}

pub(super) fn image_inline_data_preview(raw: &str, label: &str) -> Value {
    let raw = raw.trim();
    if let Some((mime_type, payload_chars)) = data_url_preview_parts(raw) {
        return json!({
            "inlineData": {
                "mimeType": mime_type,
                "data": format!("[已省略内联媒体数据: label={label}, payload_chars={payload_chars}]"),
            }
        });
    }
    json!({
        "inlineData": {
            "mimeType": "image/png",
            "data": format!("[预览媒体占位: label={label}, source={}]", preview_media_source(raw)),
        }
    })
}

pub(super) fn data_url_preview_parts(raw: &str) -> Option<(&str, usize)> {
    let rest = raw.strip_prefix("data:")?;
    let (metadata, data) = rest.split_once(',')?;
    let mime_type = metadata
        .split(';')
        .next()
        .map(str::trim)
        .filter(|value| value.starts_with("image/"))?;
    Some((mime_type, data.trim().chars().count()))
}

pub(super) fn preview_media_source(raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() {
        return "<empty>".to_string();
    }
    let limit = 160;
    let prefix = raw.chars().take(limit).collect::<String>();
    if raw.chars().count() <= limit {
        prefix
    } else {
        format!("{prefix}...")
    }
}
