use serde_json::{Map, Value};

pub(in super::super) fn validate_json_object_keys(
    object: &Map<String, Value>,
    allowed_keys: &[&str],
    path: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    for key in object.keys() {
        ensure_package!(
            allowed_keys.contains(&key.as_str()),
            "项目族导入包 {path}.{key} 是未知字段: {owner_kind}_id={}",
            owner_id
        );
    }
    Ok(())
}

pub(in super::super) fn validate_required_json_string_value(
    object: &Map<String, Value>,
    key: &str,
    expected: &str,
    path: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    let value = object.get(key).and_then(Value::as_str).ok_or_else(|| {
        format!("项目族导入包 {path}.{key} 必须是字符串: {owner_kind}_id={owner_id}")
    })?;
    ensure_package!(
        value == expected,
        "项目族导入包 {path}.{key} 无效: {owner_kind}_id={}, {key}={}",
        owner_id,
        value
    );
    Ok(())
}

pub(in super::super) fn validate_optional_json_string_or_null(
    object: &Map<String, Value>,
    key: &str,
    path: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    let Some(value) = object.get(key) else {
        return Ok(());
    };
    if value.is_null() || value.is_string() {
        return Ok(());
    }
    Err(format!(
        "项目族导入包 {path}.{key} 必须是字符串或 null: {owner_kind}_id={owner_id}"
    ))
}
