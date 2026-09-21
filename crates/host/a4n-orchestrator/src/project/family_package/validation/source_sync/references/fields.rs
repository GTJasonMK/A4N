use super::super::contracts::SourceSyncReferenceSets;
use serde_json::{Map, Value};
use std::collections::HashSet;

pub(super) fn validate_optional_source_sync_json_id_ref(
    object: &Map<String, Value>,
    key: &str,
    valid_ids: &HashSet<i64>,
    label: &str,
    json_label: &str,
    conflict_id: i64,
) -> Result<Option<i64>, String> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value.as_i64().ok_or_else(|| {
        format!(
            "项目族导入包源同步冲突 {json_label}.{key} 必须是整数: source_sync_conflict_id={conflict_id}"
        )
    })?;
    ensure_package!(
        valid_ids.contains(&value),
        "项目族导入包源同步冲突 {json_label}.{key} 引用的{label}不存在: source_sync_conflict_id={}, {key}={}",
        conflict_id,
        value
    );
    Ok(Some(value))
}

pub(super) fn validate_optional_source_sync_profile_ref(
    object: &Map<String, Value>,
    json_label: &str,
    conflict_id: i64,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    let Some(value) = object.get("source_profile_id") else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let profile_kind = object
        .get("profile_kind")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!(
                "项目族导入包源同步冲突 {json_label}.profile_kind 必须是字符串: source_sync_conflict_id={conflict_id}"
            )
        })?;
    let (valid_ids, label) = match profile_kind {
        "character" => (&refs.character_profile_ids, "角色档案"),
        "object" => (&refs.object_profile_ids, "物体档案"),
        "scene" => (&refs.scene_profile_ids, "场景档案"),
        other => {
            return Err(format!(
                "项目族导入包源同步冲突 {json_label}.profile_kind 无效: source_sync_conflict_id={}, profile_kind={other}",
                conflict_id
            ));
        }
    };
    validate_optional_source_sync_json_id_ref(
        object,
        "source_profile_id",
        valid_ids,
        label,
        json_label,
        conflict_id,
    )?;
    Ok(())
}
