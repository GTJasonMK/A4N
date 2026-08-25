use serde_json::Value;

pub(super) fn optional_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn collect_json_strings(value: Option<&Value>, limit: usize) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }

    match value {
        Some(Value::Array(items)) => array_strings(items, limit),
        Some(Value::Object(map)) => map_strings(map, limit),
        Some(Value::String(text)) => {
            let text = text.trim();
            (!text.is_empty())
                .then(|| text.to_owned())
                .into_iter()
                .collect()
        }
        _ => Vec::new(),
    }
}

fn array_strings(items: &[Value], limit: usize) -> Vec<String> {
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
            Value::Object(object) => {
                let parts = [
                    "name",
                    "title",
                    "description",
                    "summary",
                    "event",
                    "state_summary",
                    "goal",
                ]
                .into_iter()
                .filter_map(|key| {
                    object
                        .get(key)
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|text| !text.is_empty())
                        .map(ToOwned::to_owned)
                })
                .collect::<Vec<_>>();
                if !parts.is_empty() {
                    out.push(parts.join("："));
                }
            }
            _ => {}
        }
    }
    out
}

fn map_strings(map: &serde_json::Map<String, Value>, limit: usize) -> Vec<String> {
    let mut out = Vec::new();
    for (key, value) in map {
        if out.len() >= limit {
            break;
        }
        let summary = if let Some(text) = value.as_str().map(str::trim).filter(|s| !s.is_empty()) {
            text.to_owned()
        } else if let Some(object) = value.as_object() {
            let bits = [
                "summary",
                "description",
                "state_summary",
                "content",
                "goal",
                "location",
            ]
            .into_iter()
            .filter_map(|field| {
                object
                    .get(field)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                    .map(ToOwned::to_owned)
            })
            .collect::<Vec<_>>();
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

pub(super) fn stringify_json_value(value: &Value, max_chars: usize) -> String {
    let text = value
        .as_str()
        .map_or_else(|| value.to_string(), ToOwned::to_owned);
    let trimmed = text.trim();
    if trimmed.chars().count() > max_chars {
        let mut truncated = trimmed.chars().take(max_chars).collect::<String>();
        truncated.push('…');
        truncated
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{collect_json_strings, optional_string, stringify_json_value};

    #[test]
    fn optional_string_trims_non_empty_values() {
        assert_eq!(
            optional_string(&json!({"name": " 林岚 "}), "name"),
            Some("林岚".into())
        );
    }

    #[test]
    fn optional_string_ignores_missing_and_blank_values() {
        let value = json!({"blank": "  "});

        assert_eq!(optional_string(&value, "missing"), None);
        assert_eq!(optional_string(&value, "blank"), None);
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
