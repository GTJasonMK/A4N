//! Strict JSON helpers for reverse-engineering import responses.

use serde_json::Value;

pub(crate) fn optional_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

pub(crate) fn required_string(value: &Value, key: &str, context: &str) -> Result<String, String> {
    optional_string(value, key).ok_or_else(|| format!("{context} 缺少必填字符串字段 {key}"))
}

pub(crate) fn optional_i32(value: &Value, key: &str, context: &str) -> Result<Option<i32>, String> {
    let Some(value) = value.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let raw = value
        .as_i64()
        .ok_or_else(|| format!("{context}.{key} 必须是整数"))?;
    i32::try_from(raw)
        .map(Some)
        .map_err(|_| format!("{context}.{key} 超出 i32 可表示范围"))
}

pub(crate) fn required_i32(value: &Value, key: &str, context: &str) -> Result<i32, String> {
    optional_i32(value, key, context)?.ok_or_else(|| format!("{context} 缺少必填整数字段 {key}"))
}

pub(crate) fn optional_array<'a>(value: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    value.get(key).and_then(Value::as_array)
}

pub(crate) fn required_array<'a>(
    value: &'a Value,
    key: &str,
    context: &str,
) -> Result<&'a Vec<Value>, String> {
    optional_array(value, key).ok_or_else(|| format!("{context} 缺少必填数组字段 {key}"))
}

pub(crate) fn optional_field_clone(value: &Value, key: &str) -> Option<Value> {
    value.get(key).filter(|v| !v.is_null()).cloned()
}

pub(crate) fn reject_unknown_keys(
    value: &Value,
    allowed: &[&str],
    context: &str,
) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{context} 必须是 JSON 对象"))?;
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!(
                "{context} 包含未知字段 {key}；允许字段：{}",
                allowed.join(", ")
            ));
        }
    }
    Ok(())
}

fn value_to_string(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => {
            let text = text.trim();
            (!text.is_empty()).then(|| text.to_owned())
        }
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn value_to_string_list(value: Option<&Value>, limit: usize) -> Vec<String> {
    let Some(Value::Array(items)) = value else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for item in items {
        if out.len() >= limit {
            break;
        }
        match item {
            Value::String(text) => {
                let text = text.trim();
                if !text.is_empty() {
                    out.push(text.to_owned());
                }
            }
            Value::Number(number) => out.push(number.to_string()),
            Value::Object(obj) => {
                let mut parts = Vec::new();
                for key in [
                    "name",
                    "title",
                    "description",
                    "summary",
                    "event",
                    "state_summary",
                    "goal",
                ] {
                    if let Some(text) = obj
                        .get(key)
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                    {
                        parts.push(text.to_owned());
                    }
                }
                if !parts.is_empty() {
                    out.push(parts.join("："));
                }
            }
            _ => {}
        }
    }
    out
}

fn map_value_to_string_list(value: Option<&Value>, limit: usize) -> Vec<String> {
    let Some(Value::Object(map)) = value else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, val) in map {
        if out.len() >= limit {
            break;
        }
        let summary = if let Some(text) = val.as_str().map(str::trim).filter(|s| !s.is_empty()) {
            text.to_owned()
        } else if let Some(obj) = val.as_object() {
            let mut bits = Vec::new();
            for field in [
                "summary",
                "description",
                "state_summary",
                "content",
                "goal",
                "location",
            ] {
                if let Some(text) = obj
                    .get(field)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    bits.push(text.to_owned());
                }
            }
            if bits.is_empty() {
                continue;
            }
            bits.join("；")
        } else {
            continue;
        };
        out.push(format!("{key}: {summary}"));
    }
    out
}

pub(crate) fn collect_json_strings(value: Option<&Value>, limit: usize) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }

    match value {
        Some(Value::Array(_)) => value_to_string_list(value, limit),
        Some(Value::Object(_)) => map_value_to_string_list(value, limit),
        Some(Value::String(_)) => value_to_string(value).into_iter().collect(),
        _ => Vec::new(),
    }
}

pub(crate) fn stringify_json_value(value: &Value, max_chars: usize) -> String {
    let text = if let Some(text) = value.as_str() {
        text.to_string()
    } else {
        value.to_string()
    };
    let trimmed = text.trim().to_string();
    if trimmed.chars().count() > max_chars {
        let mut truncated = trimmed.chars().take(max_chars).collect::<String>();
        truncated.push('…');
        truncated
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn required_string_rejects_missing_or_empty_values() {
        let value = json!({"title": "  "});

        assert!(required_string(&value, "title", "reverse_blueprint").is_err());
        assert!(required_string(&value, "genre", "reverse_blueprint").is_err());
    }

    #[test]
    fn required_i32_does_not_parse_string_numbers() {
        let value = json!({"chapter_number": "1"});

        assert!(required_i32(&value, "chapter_number", "reverse_outline").is_err());
    }

    #[test]
    fn optional_i32_rejects_present_non_integer_values() {
        let value = json!({"total_chapters": "50"});

        assert!(optional_i32(&value, "total_chapters", "reverse_blueprint").is_err());
    }

    #[test]
    fn required_array_requires_canonical_key() {
        let value = json!({"chapter_outlines": []});

        assert!(required_array(&value, "outlines", "reverse_outline").is_err());
    }

    #[test]
    fn reject_unknown_keys_rejects_extra_fields() {
        let value = json!({"outlines": [], "legacy": true});

        let err = reject_unknown_keys(&value, &["outlines"], "reverse_outline")
            .expect_err("unknown keys must fail");

        assert!(err.contains("未知字段 legacy"));
    }

    #[test]
    fn collect_json_strings_respects_zero_limit_for_scalar_strings() {
        let value = json!("摘要");

        assert!(collect_json_strings(Some(&value), 0).is_empty());
    }

    #[test]
    fn stringify_json_value_truncates_without_alternate_serialization() {
        let value = json!({"summary": "abcdef"});

        assert_eq!(stringify_json_value(&value, 10), "{\"summary\"…");
    }
}
