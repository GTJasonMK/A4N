use a4n_core::{source_spans::source_text_hash, video_execution::VideoSegmentExecutionRecipeV1};
use a4n_domain::clear_canvas_transfer_runtime_identity;
use serde_json::Value;

use super::{json_object, remap_object_id, remap_profile_source_id};
use crate::project_family_import::{ImportIdMaps, ProjectFamilyImportError, map_required};

pub(in crate::project_family_import) fn remap_chapter_video_reference_image_ids(
    value: Option<&Value>,
    maps: &ImportIdMaps,
) -> Result<Option<Value>, ProjectFamilyImportError> {
    value
        .map(|value| remap_id_array(value, &maps.generated_image_ids, "章节视频提示词参考图片"))
        .transpose()
}

pub(in crate::project_family_import) fn remap_video_metadata(
    value: Option<&Value>,
    maps: &ImportIdMaps,
) -> Result<Option<Value>, ProjectFamilyImportError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let mut object = json_object(value, "视频 metadata_json")?;
    remap_video_metadata_object(&mut object, maps)?;
    let mut value = Value::Object(object);
    clear_canvas_transfer_runtime_identity(&mut value);
    Ok(Some(value))
}

pub(in crate::project_family_import) fn remap_video_request_snapshot_fields(
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    remap_video_metadata_object(object, maps)
}

fn remap_video_metadata_object(
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    for key in [
        "prompt_id",
        "chapter_video_prompt_id",
        "segment_plan_prompt_id",
    ] {
        remap_object_id(
            object,
            key,
            &maps.chapter_video_prompt_ids,
            "视频 metadata_json",
        )?;
    }
    for key in [
        "generated_image_id",
        "source_image_id",
        "start_frame_image_id",
        "first_frame_image_id",
        "last_frame_image_id",
    ] {
        remap_object_id(object, key, &maps.generated_image_ids, "视频 metadata_json")?;
    }
    for key in ["generated_video_id", "source_video_id"] {
        remap_object_id(object, key, &maps.generated_video_ids, "视频 metadata_json")?;
    }
    remap_object_id(
        object,
        "shooting_script_id",
        &maps.shooting_script_ids,
        "视频 metadata_json",
    )?;
    for key in [
        "generation_reference_image_ids",
        "prompt_reference_image_ids",
        "provider_reference_image_ids",
        "reference_image_ids",
        "video_library_image_ids",
    ] {
        remap_optional_id_array_field(
            object,
            key,
            &maps.generated_image_ids,
            "视频 metadata_json",
        )?;
    }
    remap_optional_id_array_field(
        object,
        "source_video_ids",
        &maps.generated_video_ids,
        "视频 metadata_json",
    )?;

    let available_assets_sha256 = remap_available_assets(object, maps)?;
    remap_generation_asset_fingerprint(
        object.get_mut("generation_asset_fingerprint"),
        maps,
        available_assets_sha256.as_deref(),
        "视频 metadata_json.generation_asset_fingerprint",
    )?;
    remap_video_execution_identity(
        object.get_mut("video_execution_identity"),
        maps,
        available_assets_sha256.as_deref(),
    )?;

    for key in [
        "segment",
        "first_frame",
        "boundary_frame",
        "start_boundary_frame",
        "end_boundary_frame",
        "start_first_frame",
        "storyboard_frame",
    ] {
        remap_reference_image_object(
            object.get_mut(key),
            maps,
            &format!("视频 metadata_json.{key}"),
        )?;
    }
    remap_reference_image_object_array(
        object.get_mut("segments"),
        maps,
        "视频 metadata_json.segments",
    )?;
    if let Some(video_unit) = object.get_mut("video_unit") {
        let video_unit = video_unit.as_object_mut().ok_or_else(|| {
            ProjectFamilyImportError::invalid_package("视频 metadata_json.video_unit 必须是对象")
        })?;
        remap_reference_image_object(
            video_unit.get_mut("prompt_segment"),
            maps,
            "视频 metadata_json.video_unit.prompt_segment",
        )?;
    }
    remap_segment_recipe(
        object.get_mut("segment_recipe"),
        maps,
        "视频 metadata_json.segment_recipe",
    )?;
    remap_segment_recipe_array(
        object.get_mut("segment_recipes"),
        maps,
        "视频 metadata_json.segment_recipes",
    )?;
    remap_reference_binding(object.get_mut("reference_binding"), maps)?;
    remap_storyboard(object.get_mut("storyboard"), maps)?;
    remap_chat_video(object.get_mut("chat_video"), maps)?;
    remap_boundary_frame_ids(object.get_mut("boundary_frame_image_ids"), maps)?;
    remap_assembly_push(object.get_mut("assembly_push"), maps)?;
    remap_postprocess_edit(object.get_mut("postprocess_edit"), maps)?;
    remap_canvas_assembly(object.get_mut("canvas_assembly"), maps)?;
    Ok(())
}

fn remap_available_assets(
    object: &mut serde_json::Map<String, Value>,
    maps: &ImportIdMaps,
) -> Result<Option<String>, ProjectFamilyImportError> {
    let Some(value) = object.get_mut("available_assets") else {
        return Ok(None);
    };
    let assets = value.as_array_mut().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package("视频 metadata_json.available_assets 必须是数组")
    })?;
    for (index, asset) in assets.iter_mut().enumerate() {
        let label = format!("视频 metadata_json.available_assets[{index}]");
        let asset = asset.as_object_mut().ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!("{label} 必须是对象"))
        })?;
        remap_object_id(asset, "image_id", &maps.generated_image_ids, &label)?;
        remap_object_id(asset, "project_id", &maps.project_ids, &label)?;
        remap_profile_source_id(asset, "profile_kind", "profile_id", maps, &label)?;
    }
    Ok(Some(source_text_hash(&value.to_string())))
}

fn remap_generation_asset_fingerprint(
    value: Option<&mut Value>,
    maps: &ImportIdMaps,
    available_assets_sha256: Option<&str>,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = value else {
        return Ok(());
    };
    let object = value
        .as_object_mut()
        .ok_or_else(|| ProjectFamilyImportError::invalid_package(format!("{label} 必须是对象")))?;
    for key in ["reference_image_ids", "video_library_image_ids"] {
        remap_optional_id_array_field(object, key, &maps.generated_image_ids, label)?;
    }
    if let Some(hash) = available_assets_sha256 {
        object.insert(
            "available_assets_sha256".to_string(),
            Value::String(hash.to_string()),
        );
    }
    Ok(())
}

fn remap_video_execution_identity(
    value: Option<&mut Value>,
    maps: &ImportIdMaps,
    available_assets_sha256: Option<&str>,
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = value else {
        return Ok(());
    };
    let object = value.as_object_mut().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(
            "视频 metadata_json.video_execution_identity 必须是对象",
        )
    })?;
    remap_object_id(
        object,
        "segment_plan_prompt_id",
        &maps.chapter_video_prompt_ids,
        "视频 metadata_json.video_execution_identity",
    )?;
    remap_generation_asset_fingerprint(
        object.get_mut("generation_asset_fingerprint"),
        maps,
        available_assets_sha256,
        "视频 metadata_json.video_execution_identity.generation_asset_fingerprint",
    )?;
    let Some(identity) = object.get_mut("shooting_script_identity") else {
        return Ok(());
    };
    let identity = identity.as_object_mut().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(
            "视频 metadata_json.video_execution_identity.shooting_script_identity 必须是对象",
        )
    })?;
    let old_id = identity
        .get("record_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(
                "视频 shooting_script_identity.record_id 缺失或不是整数",
            )
        })?;
    identity.insert(
        "record_id".to_string(),
        Value::from(map_required(
            &maps.shooting_script_ids,
            old_id,
            "视频 shooting_script_identity.record_id",
        )?),
    );
    identity.insert(
        "source_revision_key".to_string(),
        Value::String(
            maps.shooting_script_revision_keys
                .get(&old_id)
                .cloned()
                .ok_or_else(|| {
                    ProjectFamilyImportError::invalid_package(format!(
                        "视频 shooting_script_identity 引用了尚未导入的拍摄剧本: {old_id}"
                    ))
                })?,
        ),
    );
    Ok(())
}

fn remap_reference_image_object(
    value: Option<&mut Value>,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let object = value
        .as_object_mut()
        .ok_or_else(|| ProjectFamilyImportError::invalid_package(format!("{label} 必须是对象")))?;
    remap_optional_id_array_field(
        object,
        "reference_image_ids",
        &maps.generated_image_ids,
        label,
    )
}

fn remap_reference_image_object_array(
    value: Option<&mut Value>,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = value else {
        return Ok(());
    };
    let values = value
        .as_array_mut()
        .ok_or_else(|| ProjectFamilyImportError::invalid_package(format!("{label} 必须是数组")))?;
    for (index, value) in values.iter_mut().enumerate() {
        remap_reference_image_object(Some(value), maps, &format!("{label}[{index}]"))?;
    }
    Ok(())
}

fn remap_segment_recipe(
    value: Option<&mut Value>,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = value else {
        return Ok(());
    };
    let mut recipe = serde_json::from_value::<VideoSegmentExecutionRecipeV1>(value.clone())
        .map_err(|error| {
            ProjectFamilyImportError::invalid_package(format!("{label} 无效: {error}"))
        })?;
    recipe.reference_image_ids = remap_ids(
        &recipe.reference_image_ids,
        &maps.generated_image_ids,
        &format!("{label}.reference_image_ids"),
    )?;
    recipe.video_library_image_ids = remap_ids(
        &recipe.video_library_image_ids,
        &maps.generated_image_ids,
        &format!("{label}.video_library_image_ids"),
    )?;
    recipe.guidance_reference_image_ids = remap_ids(
        &recipe.guidance_reference_image_ids,
        &maps.generated_image_ids,
        &format!("{label}.guidance_reference_image_ids"),
    )?;
    recipe.continuity_source_video_id = recipe
        .continuity_source_video_id
        .map(|id| map_required(&maps.generated_video_ids, id, label))
        .transpose()?;
    recipe.adopted_video_id = recipe
        .adopted_video_id
        .map(|id| map_required(&maps.generated_video_ids, id, label))
        .transpose()?;
    *value = serde_json::to_value(recipe).map_err(|error| {
        ProjectFamilyImportError::invalid_package(format!("{label} 序列化失败: {error}"))
    })?;
    Ok(())
}

fn remap_segment_recipe_array(
    value: Option<&mut Value>,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = value else {
        return Ok(());
    };
    let values = value
        .as_array_mut()
        .ok_or_else(|| ProjectFamilyImportError::invalid_package(format!("{label} 必须是数组")))?;
    for (index, value) in values.iter_mut().enumerate() {
        remap_segment_recipe(Some(value), maps, &format!("{label}[{index}]"))?;
    }
    Ok(())
}

fn remap_reference_binding(
    value: Option<&mut Value>,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = value else {
        return Ok(());
    };
    let object = value.as_object_mut().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package("视频 metadata_json.reference_binding 必须是对象")
    })?;
    let Some(bindings) = object.get_mut("bindings") else {
        return Ok(());
    };
    let bindings = bindings.as_array_mut().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(
            "视频 metadata_json.reference_binding.bindings 必须是数组",
        )
    })?;
    for (index, binding) in bindings.iter_mut().enumerate() {
        let binding = binding.as_object_mut().ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!(
                "视频 metadata_json.reference_binding.bindings[{index}] 必须是对象"
            ))
        })?;
        remap_object_id(
            binding,
            "image_id",
            &maps.generated_image_ids,
            "视频 metadata_json.reference_binding.bindings",
        )?;
    }
    Ok(())
}

fn remap_storyboard(
    value: Option<&mut Value>,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = value else {
        return Ok(());
    };
    let object = value.as_object_mut().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package("视频 metadata_json.storyboard 必须是对象")
    })?;
    remap_object_id(
        object,
        "image_id",
        &maps.generated_image_ids,
        "视频 metadata_json.storyboard",
    )
}

fn remap_chat_video(
    value: Option<&mut Value>,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = value else {
        return Ok(());
    };
    let object = value.as_object_mut().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package("视频 metadata_json.chat_video 必须是对象")
    })?;
    let Some(images) = object.get_mut("input_images") else {
        return Ok(());
    };
    let images = images.as_array_mut().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(
            "视频 metadata_json.chat_video.input_images 必须是数组",
        )
    })?;
    for (index, image) in images.iter_mut().enumerate() {
        let image = image.as_object_mut().ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!(
                "视频 metadata_json.chat_video.input_images[{index}] 必须是对象"
            ))
        })?;
        remap_object_id(
            image,
            "image_id",
            &maps.generated_image_ids,
            "视频 metadata_json.chat_video.input_images",
        )?;
    }
    Ok(())
}

fn remap_boundary_frame_ids(
    value: Option<&mut Value>,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = value else {
        return Ok(());
    };
    let object = value.as_object_mut().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(
            "视频 metadata_json.boundary_frame_image_ids 必须是对象",
        )
    })?;
    for key in ["first", "last"] {
        remap_object_id(
            object,
            key,
            &maps.generated_image_ids,
            "视频 metadata_json.boundary_frame_image_ids",
        )?;
    }
    Ok(())
}

fn remap_assembly_push(
    value: Option<&mut Value>,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = value else {
        return Ok(());
    };
    let object = value.as_object_mut().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package("视频 metadata_json.assembly_push 必须是对象")
    })?;
    remap_optional_id_array_field(
        object,
        "source_video_ids",
        &maps.generated_video_ids,
        "视频 metadata_json.assembly_push",
    )
}

fn remap_postprocess_edit(
    value: Option<&mut Value>,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = value else {
        return Ok(());
    };
    let object = value.as_object_mut().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package("视频 metadata_json.postprocess_edit 必须是对象")
    })?;
    for key in ["push_video_id", "base_video_id", "source_video_id"] {
        remap_object_id(
            object,
            key,
            &maps.generated_video_ids,
            "视频 metadata_json.postprocess_edit",
        )?;
    }
    Ok(())
}

fn remap_canvas_assembly(
    value: Option<&mut Value>,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = value else {
        return Ok(());
    };
    let object = value.as_object_mut().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package("视频 metadata_json.canvas_assembly 必须是对象")
    })?;
    remap_optional_id_array_field(
        object,
        "source_video_ids",
        &maps.generated_video_ids,
        "视频 metadata_json.canvas_assembly",
    )
}

fn remap_optional_id_array_field(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    map: &std::collections::HashMap<i64, i64>,
    label: &str,
) -> Result<(), ProjectFamilyImportError> {
    let Some(value) = object.get_mut(key) else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    *value = remap_id_array(value, map, &format!("{label}.{key}"))?;
    Ok(())
}

fn remap_id_array(
    value: &Value,
    map: &std::collections::HashMap<i64, i64>,
    label: &str,
) -> Result<Value, ProjectFamilyImportError> {
    let values = value.as_array().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package(format!("{label} 必须是整数 ID 数组"))
    })?;
    Ok(Value::Array(
        values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let old_id = value.as_i64().ok_or_else(|| {
                    ProjectFamilyImportError::invalid_package(format!(
                        "{label}[{index}] 必须是整数 ID"
                    ))
                })?;
                Ok(Value::from(map_required(map, old_id, label)?))
            })
            .collect::<Result<Vec<_>, ProjectFamilyImportError>>()?,
    ))
}

fn remap_ids(
    ids: &[i64],
    map: &std::collections::HashMap<i64, i64>,
    label: &str,
) -> Result<Vec<i64>, ProjectFamilyImportError> {
    ids.iter().map(|id| map_required(map, *id, label)).collect()
}
