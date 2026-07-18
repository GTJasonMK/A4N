use super::super::{FreeformCanvasCopyMaps, FreeformCopyMaps};
use crate::project::duplicate::ProjectDuplicateError;
use serde_json::Value;
use std::collections::HashMap;

pub(in crate::project::duplicate::freeform::remap) fn remap_freeform_canvas_json_ids(
    value: &mut Value,
    maps: &FreeformCopyMaps,
    canvas_maps: Option<&FreeformCanvasCopyMaps>,
) -> Result<(), ProjectDuplicateError> {
    let Some(object) = value.as_object_mut() else {
        return Ok(());
    };
    remap_json_i64_field(object, "canvas_text_asset_id", &maps.text_asset_ids)?;
    remap_json_i64_field(object, "generation_spec_id", &maps.generation_spec_ids)?;
    remap_json_i64_field(object, "prompt_artifact_id", &maps.prompt_artifact_ids)?;
    remap_json_i64_field(object, "generated_image_id", &maps.images.ids)?;
    remap_json_i64_field(object, "generated_video_id", &maps.videos.ids)?;
    remap_json_i64_field(object, "image_id", &maps.images.ids)?;
    remap_json_i64_field(object, "video_id", &maps.videos.ids)?;
    remap_json_i64_array_field(object, "video_ids", &maps.videos.ids)?;
    remap_json_i64_field(object, "source_video_id", &maps.videos.ids)?;
    remap_json_i64_field(object, "source_image_id", &maps.images.ids)?;
    remap_json_i64_field(object, "first_frame_image_id", &maps.images.ids)?;
    remap_json_i64_array_field(object, "reference_image_ids", &maps.images.ids)?;
    remap_json_i64_field(object, "media_asset_id", &maps.media_asset_ids)?;
    remap_json_i64_array_field(object, "media_asset_ids", &maps.media_asset_ids)?;
    remap_json_i64_field(object, "preview_media_asset_id", &maps.media_asset_ids)?;
    remap_json_i64_field(object, "video_segment_id", &maps.video_segment_ids)?;
    remap_json_i64_array_field(object, "video_segment_ids", &maps.video_segment_ids)?;
    remap_json_i64_field(object, "source_segment_id", &maps.video_segment_ids)?;
    remap_json_i64_field(object, "assembly_timeline_id", &maps.assembly_timeline_ids)?;
    remap_json_i64_array_field(object, "assembly_clip_ids", &maps.assembly_clip_ids)?;
    if let Some(canvas_maps) = canvas_maps {
        remap_json_i64_field(object, "graph_id", &canvas_maps.graph_ids)?;
        remap_json_i64_field(object, "canvas_graph_id", &canvas_maps.graph_ids)?;
        remap_json_i64_field(object, "node_id", &canvas_maps.node_ids)?;
        remap_json_i64_field(object, "canvas_node_id", &canvas_maps.node_ids)?;
    } else {
        remap_json_i64_field(object, "graph_id", &maps.graph_ids)?;
        remap_json_i64_field(object, "canvas_graph_id", &maps.graph_ids)?;
        remap_json_i64_field(object, "node_id", &maps.node_ids)?;
        remap_json_i64_field(object, "canvas_node_id", &maps.node_ids)?;
    }
    Ok(())
}

fn remap_json_i64_field(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    map: &HashMap<i64, i64>,
) -> Result<(), ProjectDuplicateError> {
    let Some(value) = object.get(key) else {
        return Ok(());
    };
    let old_id = value.as_i64().ok_or_else(|| {
        ProjectDuplicateError::InvalidArgument(format!("复制自由视频项目字段 {key} 必须是整数 ID"))
    })?;
    let new_id = map
        .get(&old_id)
        .copied()
        .ok_or_else(|| ProjectDuplicateError::InvalidState {
            current: format!("{key}={old_id} 缺少目标映射"),
            operation: "复制自由视频项目 JSON 引用".to_string(),
        })?;
    object.insert(key.to_string(), Value::Number(new_id.into()));
    Ok(())
}

fn remap_json_i64_array_field(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    map: &HashMap<i64, i64>,
) -> Result<(), ProjectDuplicateError> {
    let Some(value) = object.get(key) else {
        return Ok(());
    };
    let values = value.as_array().ok_or_else(|| {
        ProjectDuplicateError::InvalidArgument(format!(
            "复制自由视频项目字段 {key} 必须是整数 ID 数组"
        ))
    })?;
    let remapped = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let old_id = value.as_i64().ok_or_else(|| {
                ProjectDuplicateError::InvalidArgument(format!(
                    "复制自由视频项目字段 {key}[{index}] 必须是整数 ID"
                ))
            })?;
            let new_id =
                map.get(&old_id)
                    .copied()
                    .ok_or_else(|| ProjectDuplicateError::InvalidState {
                        current: format!("{key}={old_id} 缺少目标映射"),
                        operation: "复制自由视频项目 JSON 引用".to_string(),
                    })?;
            Ok(Value::Number(new_id.into()))
        })
        .collect::<Result<Vec<_>, ProjectDuplicateError>>()?;
    object.insert(key.to_string(), Value::Array(remapped));
    Ok(())
}
