use crate::project::family::import::{
    ImportIdMaps, ProjectFamilyImportError, map_required, remap_profile_id,
};
use serde_json::Value;
use std::collections::HashMap;

pub(super) fn remap_profile_source_id(
    object: &mut serde_json::Map<String, Value>,
    kind_key: &str,
    id_key: &str,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = object.get(id_key) else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let profile_kind = object
        .get(kind_key)
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!("{label}.{kind_key} 必须是字符串"))
        })?;
    let old_id = value.as_i64().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(format!("{label}.{id_key} 必须是整数"))
    })?;
    object.insert(
        id_key.to_string(),
        Value::from(remap_profile_id(
            profile_kind,
            old_id,
            maps,
            &format!("{label}.{id_key}"),
        )?),
    );
    Ok(())
}

pub(super) fn remap_object_id(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    map: &HashMap<i64, i64>,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = object.get_mut(key) else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let old_id = value.as_i64().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(format!("{label}.{key} 必须是整数"))
    })?;
    *value = Value::from(map_required(map, old_id, &format!("{label}.{key}"))?);
    Ok(())
}

pub(super) fn remap_object_ids(
    object: &mut serde_json::Map<String, Value>,
    label: &str,
    fields: &[(&str, &HashMap<i64, i64>)],
) -> Result<(), ProjectFamilyImportError> {
    for (key, map) in fields {
        remap_object_id(object, key, map, label)?;
    }
    Ok(())
}

pub(super) fn remap_required_object_id(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    map: &HashMap<i64, i64>,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    let value = object
        .get_mut(key)
        .ok_or_else(|| ProjectFamilyImportError::invalid_package(format!("{label}.{key} 缺失")))?;
    let old_id = value.as_i64().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(format!("{label}.{key} 必须是整数"))
    })?;
    *value = Value::from(map_required(map, old_id, &format!("{label}.{key}"))?);
    Ok(())
}

pub(super) fn json_object(
    value: &Value,
    label: &str,
) -> Result<serde_json::Map<String, Value>, ProjectFamilyImportError> {
    value.as_object().cloned().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(format!("{label} 必须是 JSON 对象"))
    })
}
