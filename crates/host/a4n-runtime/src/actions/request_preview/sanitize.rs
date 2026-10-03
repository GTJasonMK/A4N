use serde_json::{Value, json};

pub(crate) fn sanitize_preview_value(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(redact_data_uris_in_text(&text)),
        Value::Array(values) => {
            Value::Array(values.into_iter().map(sanitize_preview_value).collect())
        }
        Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (key, value) in map {
                if let Value::String(text) = &value
                    && let Some(summary) = inline_media_summary(text)
                {
                    out.insert(format!("{key}_omitted"), summary);
                    continue;
                }
                out.insert(key, sanitize_preview_value(value));
            }
            Value::Object(out)
        }
        other => other,
    }
}

fn inline_media_summary(text: &str) -> Option<Value> {
    let text = text.trim();
    let (mime, payload_chars, payload_end) = data_uri_parts(text)?;
    if payload_end != text.len() {
        return None;
    }
    Some(json!({
        "kind": "inline_media",
        "mime": mime,
        "payload_chars": payload_chars,
        "omitted": true,
        "note": "预览已省略完整内联媒体数据；真实请求按对应 provider 的传输方式处理"
    }))
}

pub(super) fn redact_data_uris_in_text(text: &str) -> String {
    if !text.contains("data:") || !text.contains(";base64,") {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len().min(1024));
    let mut cursor = 0;
    while let Some(relative_start) = text[cursor..].find("data:") {
        let start = cursor + relative_start;
        out.push_str(&text[cursor..start]);
        if text[start..].find(',').is_none() {
            out.push_str(&text[start..]);
            cursor = text.len();
            break;
        }
        let Some((mime, payload_chars, payload_end)) = data_uri_parts(&text[start..]) else {
            out.push_str("data:");
            cursor = start + "data:".len();
            continue;
        };
        out.push_str(&format!(
            "[已省略内联媒体数据: mime={mime}, payload_chars={payload_chars}]"
        ));
        cursor = start + payload_end;
    }
    if cursor < text.len() {
        out.push_str(&text[cursor..]);
    }
    out
}

fn data_uri_parts(text: &str) -> Option<(&str, usize, usize)> {
    let rest = text.strip_prefix("data:")?;
    let (header_rest, _) = rest.split_once(',')?;
    let header_len = "data:".len() + header_rest.len();
    let header = &text[..header_len];
    if !header
        .split(';')
        .any(|part| part.eq_ignore_ascii_case("base64"))
    {
        return None;
    }
    let mime = header
        .strip_prefix("data:")
        .and_then(|rest| rest.split(';').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    let payload_start = header_len + 1;
    let payload_end = text[payload_start..]
        .find(|ch: char| !is_base64_payload_char(ch))
        .map(|relative_end| payload_start + relative_end)
        .unwrap_or(text.len());
    let payload_chars = text[payload_start..payload_end].chars().count();
    Some((mime, payload_chars, payload_end))
}

fn is_base64_payload_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '+' | '/' | '=')
}
