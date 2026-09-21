use serde_json::{Map, Value};
use std::collections::HashSet;

pub(in super::super) fn validate_json_id_ref(
    object: &Map<String, Value>,
    key: &str,
    valid_ids: &HashSet<i64>,
    label: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<i64, String> {
    let value = object.get(key).and_then(Value::as_i64).ok_or_else(|| {
        format!("项目族导入包 regenerate_scene {key} 必须是整数: {owner_kind}_id={owner_id}")
    })?;
    ensure_package!(
        valid_ids.contains(&value),
        "项目族导入包 regenerate_scene {key} 引用的{label}不存在: {owner_kind}_id={}, {key}={}",
        owner_id,
        value
    );
    Ok(value)
}

pub(in super::super) fn validate_optional_json_id_ref(
    object: &Map<String, Value>,
    key: &str,
    valid_ids: &HashSet<i64>,
    label: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<Option<i64>, String> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value.as_i64().ok_or_else(|| {
        format!("项目族导入包 regenerate_scene {key} 必须是整数: {owner_kind}_id={owner_id}")
    })?;
    ensure_package!(
        valid_ids.contains(&value),
        "项目族导入包 regenerate_scene {key} 引用的{label}不存在: {owner_kind}_id={}, {key}={}",
        owner_id,
        value
    );
    Ok(Some(value))
}

pub(in super::super) fn validate_required_non_negative_json_count(
    object: &Map<String, Value>,
    key: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<u64, String> {
    let value = object.get(key).ok_or_else(|| {
        format!("项目族导入包 regenerate_scene result.{key} 缺失: {owner_kind}_id={owner_id}")
    })?;
    value.as_u64().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene result.{key} 必须是非负整数: {owner_kind}_id={owner_id}"
        )
    })
}

pub(in super::super) fn validate_required_result_json_id_ref(
    object: &Map<String, Value>,
    key: &str,
    valid_ids: &HashSet<i64>,
    label: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<i64, String> {
    let value = object.get(key).and_then(Value::as_i64).ok_or_else(|| {
        format!("项目族导入包 regenerate_scene result.{key} 必须是整数: {owner_kind}_id={owner_id}")
    })?;
    ensure_package!(
        valid_ids.contains(&value),
        "项目族导入包 regenerate_scene result.{key} 引用的{label}不存在: {owner_kind}_id={}, {key}={}",
        owner_id,
        value
    );
    Ok(value)
}
