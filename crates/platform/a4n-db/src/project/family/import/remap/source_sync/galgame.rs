use super::super::super::{ImportIdMaps, ProjectFamilyImportError, map_required};
use super::super::{remap_object_id, remap_object_ids};
use serde_json::Value;

pub(super) fn remap_regenerate_scene_payload(
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    remap_object_id(object, "galgame_scene_id", &maps.galgame_scene_ids, label)?;
    let Some(result_value) = object.get_mut("result") else {
        return Ok(());
    };
    let result = result_value.as_object_mut().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(format!("{label}.result 必须是 JSON 对象"))
    })?;
    remap_object_ids(
        result,
        label,
        &[
            ("galgame_scene_id", &maps.galgame_scene_ids),
            (
                "old_source_snapshot_chapter_id",
                &maps.source_snapshot_chapter_ids,
            ),
            (
                "new_source_snapshot_chapter_id",
                &maps.source_snapshot_chapter_ids,
            ),
        ],
    )?;
    if let Some(stage_outputs) = result.get_mut("stage_outputs")
        && !stage_outputs.is_null()
    {
        remap_galgame_stage_outputs(
            stage_outputs,
            maps,
            &format!("{label}.result.stage_outputs"),
        )?;
    }
    Ok(())
}

fn remap_galgame_stage_outputs(
    value: &mut Value,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    match value {
        Value::Object(object) => {
            remap_object_ids(
                object,
                label,
                &[
                    ("text_anchor_id", &maps.text_anchor_ids),
                    ("speaker_profile_id", &maps.character_profile_ids),
                    ("scene_profile_id", &maps.scene_profile_ids),
                    (
                        "source_snapshot_chapter_id",
                        &maps.source_snapshot_chapter_ids,
                    ),
                    ("source_chapter_id", &maps.chapter_ids),
                    ("galgame_scene_id", &maps.galgame_scene_ids),
                    ("galgame_line_id", &maps.galgame_line_ids),
                ],
            )?;
            remap_source_span_id_array(object, "source_span_ids", maps, label)?;
            for nested in object.values_mut() {
                remap_galgame_stage_outputs(nested, maps, label)?;
            }
        }
        Value::Array(items) => {
            for item in items {
                remap_galgame_stage_outputs(item, maps, label)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn remap_source_span_id_array(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = object.get_mut(key) else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let items = value.as_array_mut().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(format!("{label}.{key} 必须是字符串数组"))
    })?;
    for item in items {
        let old = item.as_str().ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!("{label}.{key} 必须是字符串数组"))
        })?;
        *item = Value::String(remap_source_span_id(old, maps, &format!("{label}.{key}"))?);
    }
    Ok(())
}

fn remap_source_span_id(
    value: &str,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<String, ProjectFamilyImportError> {
    let Some((chapter_prefix, anchor_id)) = value.rsplit_once(":ta") else {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "{label} 格式无效: {value}"
        )));
    };
    if !chapter_prefix.starts_with("ch") || chapter_prefix.len() <= 2 {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "{label} 格式无效: {value}"
        )));
    }
    let old_anchor_id = anchor_id.parse::<i64>().map_err(|_| {
        ProjectFamilyImportError::invalid_package(format!("{label} 文本锚点 ID 无效: {value}"))
    })?;
    let new_anchor_id = map_required(&maps.text_anchor_ids, old_anchor_id, label)?;
    Ok(format!("{chapter_prefix}:ta{new_anchor_id}"))
}
