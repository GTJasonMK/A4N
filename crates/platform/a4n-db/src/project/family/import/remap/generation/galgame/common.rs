use super::super::super::json_ids::remap_object_ids;
use crate::project::family::import::{ProjectFamilyImportError, map_required};
use a4n_db_models::models::generation::generation_spec;
use serde_json::Value;
use std::collections::HashMap;

pub(super) fn required_mapped_target_entity_id(
    spec: &generation_spec::Model,
    map: &HashMap<i64, i64>,
    label: &str,
) -> Result<i64, ProjectFamilyImportError> {
    let old_id = spec
        .target_entity_id
        .ok_or_else(|| ProjectFamilyImportError::invalid_package(format!("{label} 缺失")))?;
    map_required(map, old_id, label)
}

pub(super) fn remap_nested_object_ids(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    label: &str,
    fields: &[(&str, &HashMap<i64, i64>)],
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = object.get_mut(key) else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let nested = value.as_object_mut().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(format!("{label} 必须是 JSON 对象"))
    })?;
    remap_object_ids(nested, label, fields)
}

pub(super) fn remap_object_array_ids(
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
    let items = value.as_array_mut().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(format!("{label} 必须是整数数组"))
    })?;
    for item in items {
        let old_id = item.as_i64().ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!("{label} 必须是整数数组"))
        })?;
        *item = Value::from(map_required(map, old_id, label)?);
    }
    Ok(())
}
