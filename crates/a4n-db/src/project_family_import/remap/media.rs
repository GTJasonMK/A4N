use super::super::{ImportIdMaps, ProjectFamilyImportError, map_required};
use super::{json_object, remap_object_id, remap_object_ids};
use a4n_core::models::media::media_binding;
use serde_json::Value;

pub fn remap_media_binding_target_id(
    binding: &media_binding::Model,
    maps: &ImportIdMaps,
) -> Result<i64, ProjectFamilyImportError> {
    match binding.target_kind.as_str() {
        "character_profile" => map_required(
            &maps.character_profile_ids,
            binding.target_id,
            "媒体绑定 character_profile target_id",
        ),
        "object_profile" => map_required(
            &maps.object_profile_ids,
            binding.target_id,
            "媒体绑定 object_profile target_id",
        ),
        "scene_profile" => map_required(
            &maps.scene_profile_ids,
            binding.target_id,
            "媒体绑定 scene_profile target_id",
        ),
        "video_segment" => map_required(
            &maps.video_segment_ids,
            binding.target_id,
            "媒体绑定 video_segment target_id",
        ),
        "chapter" => map_required(
            &maps.chapter_ids,
            binding.target_id,
            "媒体绑定 chapter target_id",
        ),
        "assembly_timeline" => map_required(
            &maps.assembly_timeline_ids,
            binding.target_id,
            "媒体绑定 assembly_timeline target_id",
        ),
        "galgame_scene" => map_required(
            &maps.galgame_scene_ids,
            binding.target_id,
            "媒体绑定 galgame_scene target_id",
        ),
        "galgame_line" => map_required(
            &maps.galgame_line_ids,
            binding.target_id,
            "媒体绑定 galgame_line target_id",
        ),
        other => Err(ProjectFamilyImportError::invalid_package(format!(
            "媒体绑定 target_kind 无效: {other}"
        ))),
    }
}
pub fn remap_media_request_snapshot(
    value: Option<&Value>,
    maps: &ImportIdMaps,
) -> Result<Option<Value>, ProjectFamilyImportError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let mut object = json_object(value, "媒体来源 request_snapshot_json")?;
    remap_object_ids(
        &mut object,
        "媒体来源 request_snapshot_json",
        &[
            ("generation_spec_id", &maps.generation_spec_ids),
            ("prompt_artifact_id", &maps.prompt_artifact_ids),
            ("timeline_id", &maps.assembly_timeline_ids),
        ],
    )?;
    remap_object_id(
        &mut object,
        "target_line_id",
        &maps.galgame_line_ids,
        "媒体来源 request_snapshot_json",
    )?;
    remap_media_request_target_entity(&mut object, maps)?;
    object.remove("config_id");
    Ok(Some(Value::Object(object)))
}

fn remap_media_request_target_entity(
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let Some(target_entity_id) = object.get("target_entity_id") else {
        return Ok(());
    };
    if target_entity_id.is_null() {
        return Ok(());
    }
    let Some(target_entity_kind) = object.get("target_entity_kind").and_then(Value::as_str) else {
        return Ok(());
    };
    let map = match target_entity_kind {
        "character_profile" => &maps.character_profile_ids,
        "galgame_scene" => &maps.galgame_scene_ids,
        "galgame_line" => &maps.galgame_line_ids,
        "segment" => &maps.video_segment_ids,
        "assembly" => &maps.assembly_timeline_ids,
        _ => return Ok(()),
    };
    remap_object_id(
        object,
        "target_entity_id",
        map,
        "媒体来源 request_snapshot_json",
    )
}
