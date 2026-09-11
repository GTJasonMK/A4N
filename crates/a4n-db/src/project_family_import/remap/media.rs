use super::super::{ImportIdMaps, ProjectFamilyImportError, map_required};
use super::generation::remap_manga_reference_bindings_value;
use super::{json_object, remap_object_id, remap_object_ids};
use a4n_db_models::models::media::media_binding;
use a4n_domain::{GeneratedMediaReference, parse_generated_media_reference};
use serde_json::Value;

pub(in crate::project_family_import) fn remap_media_binding_target_id(
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
        "chapter_video_prompt" => map_required(
            &maps.chapter_video_prompt_ids,
            binding.target_id,
            "媒体绑定 chapter_video_prompt target_id",
        ),
        "generated_video" => map_required(
            &maps.generated_video_ids,
            binding.target_id,
            "媒体绑定 generated_video target_id",
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
pub(in crate::project_family_import) fn remap_media_request_snapshot(
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
            ("chapter_id", &maps.chapter_ids),
        ],
    )?;
    super::video::remap_video_request_snapshot_fields(&mut object, maps)?;
    remap_object_id(
        &mut object,
        "target_line_id",
        &maps.galgame_line_ids,
        "媒体来源 request_snapshot_json",
    )?;
    remap_media_request_target_entity(&mut object, maps)?;
    let mut value = Value::Object(object);
    a4n_domain::clear_canvas_transfer_runtime_identity(&mut value);
    Ok(Some(value))
}

pub(in crate::project_family_import) fn remap_manga_generated_image_metadata(
    value: &Value,
    maps: &ImportIdMaps,
) -> Result<Value, ProjectFamilyImportError> {
    let mut object = json_object(value, "漫画生成图片 metadata_json")?;
    let page_prompt_id = object
        .get("page_prompt_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(
                "漫画生成图片 metadata_json.page_prompt_id 缺失或不是整数",
            )
        })?;
    object.insert(
        "page_prompt_id".to_string(),
        Value::from(map_required(
            &maps.chapter_manga_prompt_ids,
            page_prompt_id,
            "漫画生成图片 metadata_json.page_prompt_id",
        )?),
    );
    let bindings = object.get("reference_bindings").ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(
            "漫画生成图片 metadata_json.reference_bindings 缺失",
        )
    })?;
    object.insert(
        "reference_bindings".to_string(),
        remap_manga_reference_bindings_value(
            bindings,
            maps,
            "漫画生成图片 metadata_json.reference_bindings",
        )?,
    );
    if let Some(generation) = object.get_mut("generation") {
        let generation = generation.as_object_mut().ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(
                "漫画生成图片 metadata_json.generation 必须是对象",
            )
        })?;
        remap_object_id(
            generation,
            "chapter_id",
            &maps.chapter_ids,
            "漫画生成图片 metadata_json.generation",
        )?;
        generation.remove("config_id");
    }
    if let Some(context) = object.get_mut("generation_context") {
        let context = context.as_object_mut().ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(
                "漫画生成图片 metadata_json.generation_context 必须是对象",
            )
        })?;
        if let Some(bindings) = context.get("reference_bindings") {
            context.insert(
                "reference_bindings".to_string(),
                remap_manga_reference_bindings_value(
                    bindings,
                    maps,
                    "漫画生成图片 metadata_json.generation_context.reference_bindings",
                )?,
            );
        }
    }
    object.remove("config_id");
    Ok(Value::Object(object))
}

pub(in crate::project_family_import) fn remap_media_source_external_path(
    value: Option<&str>,
    maps: &ImportIdMaps,
) -> Result<Option<String>, ProjectFamilyImportError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let reference = parse_generated_media_reference(value).map_err(|error| {
        ProjectFamilyImportError::invalid_package(format!(
            "媒体来源 source_external_path 的 {} ID 无效: {value}",
            error.kind()
        ))
    })?;
    let Some(reference) = reference else {
        return Ok(Some(value.to_string()));
    };
    let new_id = match reference {
        GeneratedMediaReference::Image(old_id) => map_required(
            &maps.generated_image_ids,
            old_id,
            "媒体来源 source_external_path generated_image ID",
        )?,
        GeneratedMediaReference::Video(old_id) => map_required(
            &maps.generated_video_ids,
            old_id,
            "媒体来源 source_external_path generated_video ID",
        )?,
    };
    Ok(Some(reference.with_id(new_id).to_string()))
}

fn remap_media_request_target_entity(
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let target_entity_id = object.get("target_entity_id");
    let target_entity_kind = object.get("target_entity_kind");
    if target_entity_id.is_none() && target_entity_kind.is_none() {
        return Ok(());
    }
    let target_entity_id = target_entity_id
        .filter(|value| !value.is_null())
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(
                "媒体来源 request_snapshot_json.target_entity_id 缺失或不是整数",
            )
        })?;
    let target_entity_kind = target_entity_kind.and_then(Value::as_str).ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(
            "媒体来源 request_snapshot_json.target_entity_kind 缺失或不是字符串",
        )
    })?;
    let map = match target_entity_kind {
        "character_profile" => &maps.character_profile_ids,
        "chapter_video_prompt" => &maps.chapter_video_prompt_ids,
        "generated_video" => &maps.generated_video_ids,
        "galgame_scene" => &maps.galgame_scene_ids,
        "galgame_line" => &maps.galgame_line_ids,
        "segment" => &maps.video_segment_ids,
        "assembly" => &maps.assembly_timeline_ids,
        other => {
            return Err(ProjectFamilyImportError::invalid_package(format!(
                "媒体来源 request_snapshot_json.target_entity_kind 无效: {other}"
            )));
        }
    };
    object.insert(
        "target_entity_id".to_string(),
        Value::from(map_required(
            map,
            target_entity_id,
            "媒体来源 request_snapshot_json.target_entity_id",
        )?),
    );
    Ok(())
}
