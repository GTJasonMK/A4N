use super::super::{FreeformCanvasCopyMaps, FreeformCopyMaps};
use super::ids::remap_freeform_canvas_json_ids;
use crate::project::duplicate::ProjectDuplicateError;
use serde_json::Value;
use std::collections::HashMap;

pub(in crate::project::duplicate::freeform) fn remap_freeform_canvas_node_refs(
    ref_type: Option<String>,
    ref_id: Option<i64>,
    mut config_json: Value,
    maps: &FreeformCopyMaps,
    canvas_maps: Option<&FreeformCanvasCopyMaps>,
) -> Result<(Option<String>, Option<i64>, Value), ProjectDuplicateError> {
    remap_freeform_canvas_json_ids(&mut config_json, maps, canvas_maps)?;
    let Some(kind) = ref_type.as_deref() else {
        return Ok((ref_type, ref_id, config_json));
    };
    match kind {
        "inline" | "config" => Ok((ref_type, ref_id, config_json)),
        "legacy_generated_image" => {
            let old_id = required_freeform_canvas_ref_id(ref_id, kind)?;
            let new_id = remap_freeform_canvas_ref_id(&maps.images.ids, old_id, kind)?;
            if let Some(file_path) = maps.images.file_paths.get(&old_id) {
                set_freeform_canvas_node_config_string(&mut config_json, "file_path", file_path);
            }
            Ok((ref_type, Some(new_id), config_json))
        }
        "legacy_generated_video" => {
            let old_id = required_freeform_canvas_ref_id(ref_id, kind)?;
            let new_id = remap_freeform_canvas_ref_id(&maps.videos.ids, old_id, kind)?;
            if let Some(file_path) = maps
                .videos
                .file_paths
                .get(&old_id)
                .and_then(|value| value.as_ref())
            {
                set_freeform_canvas_node_config_string(&mut config_json, "file_path", file_path);
            }
            if let Some(thumbnail_path) = maps
                .videos
                .thumbnail_paths
                .get(&old_id)
                .and_then(|value| value.as_ref())
            {
                set_freeform_canvas_node_config_string(
                    &mut config_json,
                    "thumbnail_path",
                    thumbnail_path,
                );
            }
            Ok((ref_type, Some(new_id), config_json))
        }
        "canvas_text_asset" => remap_node_ref(ref_type, ref_id, config_json, &maps.text_asset_ids),
        "generation_spec" => {
            remap_node_ref(ref_type, ref_id, config_json, &maps.generation_spec_ids)
        }
        "prompt_artifact" => {
            remap_node_ref(ref_type, ref_id, config_json, &maps.prompt_artifact_ids)
        }
        "media_asset" => remap_node_ref(ref_type, ref_id, config_json, &maps.media_asset_ids),
        "video_segment" => remap_node_ref(ref_type, ref_id, config_json, &maps.video_segment_ids),
        "assembly_timeline" => {
            remap_node_ref(ref_type, ref_id, config_json, &maps.assembly_timeline_ids)
        }
        "legacy_chapter_video_prompt" => Err(ProjectDuplicateError::InvalidArgument(format!(
            "自由视频项目复制暂不支持重映射画布节点 {kind} 引用，不能保留源项目 ref_id: {}",
            ref_id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "NULL".to_string())
        ))),
        other => Err(ProjectDuplicateError::InvalidArgument(format!(
            "自由视频项目复制遇到未知画布节点引用类型: {other}"
        ))),
    }
}

fn remap_node_ref(
    ref_type: Option<String>,
    ref_id: Option<i64>,
    config_json: Value,
    map: &HashMap<i64, i64>,
) -> Result<(Option<String>, Option<i64>, Value), ProjectDuplicateError> {
    let kind = ref_type.as_deref().unwrap_or_default();
    let old_id = required_freeform_canvas_ref_id(ref_id, kind)?;
    let new_id = remap_freeform_canvas_ref_id(map, old_id, kind)?;
    Ok((ref_type, Some(new_id), config_json))
}

fn required_freeform_canvas_ref_id(
    ref_id: Option<i64>,
    ref_type: &str,
) -> Result<i64, ProjectDuplicateError> {
    ref_id.ok_or_else(|| ProjectDuplicateError::InvalidState {
        current: format!("画布节点 {ref_type} 引用缺少 ref_id"),
        operation: "复制自由视频项目画布节点".to_string(),
    })
}

fn remap_freeform_canvas_ref_id(
    map: &HashMap<i64, i64>,
    old_id: i64,
    ref_type: &str,
) -> Result<i64, ProjectDuplicateError> {
    map.get(&old_id)
        .copied()
        .ok_or_else(|| ProjectDuplicateError::InvalidState {
            current: format!("画布节点 {ref_type} 引用 {old_id} 缺少目标映射"),
            operation: "复制自由视频项目画布节点".to_string(),
        })
}

fn set_freeform_canvas_node_config_string(config_json: &mut Value, key: &str, value: &str) {
    if let Value::Object(object) = config_json {
        object.insert(key.to_string(), Value::String(value.to_string()));
    }
}
