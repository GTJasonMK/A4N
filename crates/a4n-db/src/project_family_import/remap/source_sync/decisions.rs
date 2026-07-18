use super::super::super::{ImportIdMaps, ProjectFamilyImportError};
use super::super::{json_object, remap_object_id};
use super::{
    galgame::remap_regenerate_scene_payload,
    reference_payload::{
        remap_anchor_mappings, remap_refresh_spec_payload, remap_replacement_segment_payload,
    },
};
use serde_json::Value;

pub fn remap_source_sync_decision_payload(
    strategy: &str,
    value: &Value,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<Value, ProjectFamilyImportError> {
    let mut object = json_object(value, label)?;
    match strategy {
        "keep_local" | "detach" | "delete_local" => {}
        "manual" => {
            remap_object_id(
                &mut object,
                "prompt_artifact_id",
                &maps.prompt_artifact_ids,
                label,
            )?;
        }
        "rebind_anchor" => {
            remap_object_id(
                &mut object,
                "generation_spec_id",
                &maps.generation_spec_ids,
                label,
            )?;
            remap_anchor_mappings(&mut object, maps, label)?;
        }
        "adopt_source" | "rebuild_prompt" => {
            remap_refresh_spec_payload(&mut object, maps, label)?;
        }
        "regenerate_scene" => {
            remap_regenerate_scene_payload(&mut object, maps, label)?;
        }
        "create_replacement" => {
            remap_refresh_spec_payload(&mut object, maps, label)?;
            remap_replacement_segment_payload(&mut object, maps, label)?;
        }
        other => {
            return Err(ProjectFamilyImportError::invalid_package(format!(
                "同步决策 strategy 无效: {other}"
            )));
        }
    }
    Ok(Value::Object(object))
}
