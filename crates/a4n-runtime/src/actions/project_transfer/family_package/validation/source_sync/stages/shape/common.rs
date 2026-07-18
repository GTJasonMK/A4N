use serde_json::{Map, Value};

pub(super) fn validate_required_stage_scene_index(
    object: &Map<String, Value>,
    path: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    validate_required_stage_index(object, "scene_index", path, owner_kind, owner_id)
}

pub(super) fn validate_required_stage_index(
    object: &Map<String, Value>,
    key: &str,
    path: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    let scene_index = object.get(key).and_then(Value::as_i64).ok_or_else(|| {
        format!("项目族导入包 regenerate_scene {path}.{key} 必须是整数: {owner_kind}_id={owner_id}")
    })?;
    ensure_package!(
        scene_index == 1,
        "项目族导入包 regenerate_scene {path}.{key} 必须为 1: {owner_kind}_id={}, {key}={}",
        owner_id,
        scene_index
    );
    Ok(())
}

pub(super) fn validate_required_positive_stage_integer(
    object: &Map<String, Value>,
    key: &str,
    path: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    let value = object.get(key).and_then(Value::as_i64).ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.{key} 必须是正整数: {owner_kind}_id={owner_id}"
        )
    })?;
    ensure_package!(
        value > 0,
        "项目族导入包 regenerate_scene {path}.{key} 必须是正整数: {owner_kind}_id={}, {key}={}",
        owner_id,
        value
    );
    Ok(())
}

pub(super) fn validate_required_nonempty_stage_source_span_ids(
    object: &Map<String, Value>,
    path: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    let source_span_ids = object
        .get("source_span_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!(
                "项目族导入包 regenerate_scene {path}.source_span_ids 必须是非空字符串数组: {owner_kind}_id={owner_id}"
            )
        })?;
    ensure_package!(
        !source_span_ids.is_empty(),
        "项目族导入包 regenerate_scene {path}.source_span_ids 必须是非空字符串数组: {owner_kind}_id={}",
        owner_id
    );
    Ok(())
}
