use serde_json::Value;

pub(in crate::sidecar_clients::request_context) fn payload_has_provider_secret(
    payload: &Value,
) -> bool {
    match payload {
        Value::Object(map) => map.iter().any(|(key, value)| {
            if key_is_provider_secret(key) {
                value_has_secret(value)
            } else {
                payload_has_provider_secret(value)
            }
        }),
        Value::Array(items) => items.iter().any(payload_has_provider_secret),
        _ => false,
    }
}

fn key_is_provider_secret(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    matches!(
        key.as_str(),
        "api_key" | "apikey" | "api_key_encrypted" | "authorization" | "bearer_token"
    ) || key.ends_with("_api_key")
        || key.ends_with("_token")
}

fn value_has_secret(value: &Value) -> bool {
    match value {
        Value::String(value) => !value.trim().is_empty(),
        Value::Null => false,
        Value::Array(items) => items.iter().any(value_has_secret),
        Value::Object(map) => map.values().any(value_has_secret),
        _ => true,
    }
}
