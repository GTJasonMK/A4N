use super::{json_object, remap_object_id};
use crate::project::family::import::{ImportIdMaps, ProjectFamilyImportError, map_required};
use serde_json::Value;

pub(in crate::project::family::import) fn remap_galgame_presentation_json(
    value: &Value,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<Value, ProjectFamilyImportError> {
    let mut object = json_object(value, label)?;
    let cast = object
        .get_mut("visible_cast")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!("{label}.visible_cast 必须是数组"))
        })?;
    for member in cast {
        let member = member.as_object_mut().ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!(
                "{label}.visible_cast 成员必须是对象"
            ))
        })?;
        super::json_ids::remap_required_object_id(
            member,
            "character_profile_id",
            &maps.character_profile_ids,
            label,
        )?;
    }
    for key in ["enter", "exit"] {
        let ids = object
            .get_mut(key)
            .and_then(Value::as_array_mut)
            .ok_or_else(|| {
                ProjectFamilyImportError::invalid_package(format!("{label}.{key} 必须是 ID 数组"))
            })?;
        for id in ids {
            let old_id = id.as_i64().ok_or_else(|| {
                ProjectFamilyImportError::invalid_package(format!("{label}.{key} 必须是 ID 数组"))
            })?;
            *id = Value::from(map_required(
                &maps.character_profile_ids,
                old_id,
                &format!("{label}.{key}"),
            )?);
        }
    }
    remap_object_id(
        &mut object,
        "focus_character_profile_id",
        &maps.character_profile_ids,
        label,
    )?;
    Ok(Value::Object(object))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn presentation_remaps_all_character_references_but_not_scene_ordinals() {
        let mut maps = ImportIdMaps::default();
        maps.character_profile_ids.extend([(7, 107), (8, 108)]);
        let presentation = json!({
            "schema_version": "galgame_line_presentation_v2",
            "visible_cast": [{"character_profile_id": 7, "expression": "neutral", "pose": "standing", "position": "left", "facing": "front", "layer": "front"}],
            "enter": [7], "exit": [8], "focus_character_profile_id": 7,
            "hold": false, "player_knowledge": [], "returns_to_scene_index": 7,
            "cg": null, "cg_note": null, "cg_importance": null, "transition": null,
            "branch": null, "branch_id": null, "after_beat_ref": null,
        });
        let mapped = remap_galgame_presentation_json(&presentation, &maps, "presentation").unwrap();
        assert_eq!(mapped["visible_cast"][0]["character_profile_id"], 107);
        assert_eq!(mapped["enter"], json!([107]));
        assert_eq!(mapped["exit"], json!([108]));
        assert_eq!(mapped["focus_character_profile_id"], 107);
        assert_eq!(mapped["returns_to_scene_index"], 7);
        assert_eq!(mapped["visible_cast"][0]["pose"], "standing");
        maps.character_profile_ids.remove(&8);
        assert!(remap_galgame_presentation_json(&presentation, &maps, "presentation").is_err());
    }
}
