use serde_json::Value;

pub(super) fn required_checkpoint_string(
    checkpoint_json: &Value,
    call_key: &str,
    field: &str,
) -> Result<String, String> {
    checkpoint_json
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{call_key}.{field} 必须是字符串"))
}

pub(super) fn optional_checkpoint_string(
    checkpoint_json: &Value,
    field: &str,
) -> Result<Option<String>, String> {
    match checkpoint_json.get(field) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => value
            .as_str()
            .map(ToString::to_string)
            .map(Some)
            .ok_or_else(|| format!("{field} 必须是字符串或 null")),
    }
}
