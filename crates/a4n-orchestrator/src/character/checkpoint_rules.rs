//! 角色同步 checkpoint JSON 字段纯校验。

use serde_json::Value;

/// checkpoint 必填字符串字段。
pub fn required_character_sync_checkpoint_string(
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

/// checkpoint 可选字符串（允许 null/缺席）。
pub fn optional_character_sync_checkpoint_string(
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn required_string_rejects_missing() {
        let err = required_character_sync_checkpoint_string(&json!({}), "k", "name").unwrap_err();
        assert!(err.contains("必须是字符串"));
    }

    #[test]
    fn optional_string_allows_null() {
        assert_eq!(
            optional_character_sync_checkpoint_string(&json!({"x": null}), "x").unwrap(),
            None
        );
        assert_eq!(
            optional_character_sync_checkpoint_string(&json!({"x": "a"}), "x").unwrap(),
            Some("a".into())
        );
    }
}
