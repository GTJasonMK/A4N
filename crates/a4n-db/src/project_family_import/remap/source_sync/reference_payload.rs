use super::super::super::{ImportIdMaps, ProjectFamilyImportError};
use super::super::{remap_object_id, remap_object_ids, remap_profile_id};
use serde_json::Value;

pub(super) fn remap_refresh_spec_payload(
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    remap_object_ids(
        object,
        label,
        &[("generation_spec_id", &maps.generation_spec_ids)],
    )?;
    remap_ref_array(object, "text_anchor_refs", label, |item| {
        remap_object_id(item, "text_anchor_id", &maps.text_anchor_ids, label)
    })?;
    remap_ref_array(object, "entity_refs", label, |item| {
        remap_entity_ref(item, maps, label)
    })?;
    remap_ref_array(object, "media_refs", label, |item| {
        remap_object_id(item, "media_asset_id", &maps.media_asset_ids, label)
    })
}

pub(super) fn remap_replacement_segment_payload(
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    let value = object.get_mut("replacement_segment").ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(format!("{label}.replacement_segment 缺失"))
    })?;
    let segment = value.as_object_mut().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(format!(
            "{label}.replacement_segment 必须是 JSON 对象"
        ))
    })?;
    remap_object_ids(
        segment,
        label,
        &[
            ("previous_segment_id", &maps.video_segment_ids),
            ("next_segment_id", &maps.video_segment_ids),
        ],
    )
}

pub(super) fn remap_anchor_mappings(
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    let mappings = object
        .get_mut("anchor_mappings")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!("{label}.anchor_mappings 必须是数组"))
        })?;
    for mapping in mappings {
        let mapping = mapping.as_object_mut().ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!(
                "{label}.anchor_mappings[] 必须是 JSON 对象"
            ))
        })?;
        remap_object_ids(
            mapping,
            label,
            &[
                ("old_text_anchor_id", &maps.text_anchor_ids),
                ("new_text_anchor_id", &maps.text_anchor_ids),
            ],
        )?;
    }
    Ok(())
}

fn remap_ref_array<F>(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    label: &str,
    remap: F,
) -> Result<(), ProjectFamilyImportError>
where
    F: FnMut(&mut serde_json::Map<String, Value>) -> Result<(), ProjectFamilyImportError>,
{
    let refs = object
        .get_mut(key)
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!("{label}.{key} 必须是数组"))
        })?;
    let mut remap = remap;
    for item in refs {
        let item = item.as_object_mut().ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!("{label}.{key}[] 必须是 JSON 对象"))
        })?;
        remap(item)?;
    }
    Ok(())
}

fn remap_entity_ref(
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    let entity_kind = object
        .get("entity_kind")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!("{label}.entity_kind 必须是字符串"))
        })?;
    let entity_id = object
        .get("entity_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!("{label}.entity_id 必须是整数"))
        })?;
    object.insert(
        "entity_id".to_string(),
        Value::from(remap_profile_id(
            entity_kind,
            entity_id,
            maps,
            &format!("{label}.entity_id"),
        )?),
    );
    Ok(())
}
