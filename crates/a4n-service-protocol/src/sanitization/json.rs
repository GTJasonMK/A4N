use serde_json::Value;

pub(super) const SENSITIVE_TEXT_KEYS: &[&str] = &[
    "api_key",
    "apikey",
    "password",
    "token",
    "secret",
    "credential",
    "authorization",
    "base_url",
    "baseurl",
];

pub fn redact_sensitive_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if is_sensitive_key(key) {
                    *child = Value::String("[REDACTED]".to_string());
                } else {
                    redact_sensitive_value(child);
                }
            }
        }
        Value::Array(items) => {
            for child in items {
                redact_sensitive_value(child);
            }
        }
        _ => {}
    }
}

pub fn contains_unredacted_sensitive_value(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, child)| {
            if is_sensitive_key(key) {
                !is_redacted_or_empty_sensitive_value(child)
            } else {
                contains_unredacted_sensitive_value(child)
            }
        }),
        Value::Array(items) => items.iter().any(contains_unredacted_sensitive_value),
        _ => false,
    }
}

fn is_redacted_or_empty_sensitive_value(value: &Value) -> bool {
    value.is_null()
        || value
            .as_str()
            .is_some_and(|value| value.trim().is_empty() || value == "[REDACTED]")
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    SENSITIVE_TEXT_KEYS
        .iter()
        .any(|sensitive| key.contains(sensitive))
}
