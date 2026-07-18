use super::super::types::{VideoStudioProjectImportError, VideoStudioProjectImportIdMaps};
use super::ids::{
    remap_video_studio_json_i64_array_field, remap_video_studio_json_i64_field,
    remap_video_studio_required_id,
};
use serde_json::Value;

pub(in crate::canvas::transfer) fn remap_video_studio_canvas_node_refs(
    ref_type: Option<String>,
    ref_id: Option<i64>,
    mut config_json: Value,
    maps: &VideoStudioProjectImportIdMaps,
) -> Result<(Option<String>, Option<i64>, Value), VideoStudioProjectImportError> {
    remap_video_studio_canvas_node_config_ids(&mut config_json, maps)?;
    let Some(kind) = ref_type.as_deref().map(str::to_string) else {
        if ref_id.is_some() {
            return Err(VideoStudioProjectImportError::InvalidPackage(
                "画布节点 ref_type 为空时不能设置 ref_id".to_string(),
            ));
        }
        return Ok((ref_type, ref_id, config_json));
    };
    match kind.as_str() {
        "inline" => {
            if ref_id.is_some() {
                return Err(VideoStudioProjectImportError::InvalidPackage(
                    "inline 画布节点不能设置 ref_id".to_string(),
                ));
            }
            Ok((ref_type, None, config_json))
        }
        "config" => Ok((ref_type, ref_id, config_json)),
        "legacy_generated_image" => {
            let old_id = required_video_studio_ref_id(ref_id, kind.as_str())?;
            let new_id =
                remap_video_studio_required_id(&maps.image_ids, old_id, "canvas_node.ref_id")?;
            if let Some(file_path) = maps.image_file_paths.get(&old_id) {
                set_video_studio_json_string(&mut config_json, "file_path", file_path);
            }
            Ok((ref_type, Some(new_id), config_json))
        }
        "legacy_generated_video" => {
            let old_id = required_video_studio_ref_id(ref_id, kind.as_str())?;
            let new_id =
                remap_video_studio_required_id(&maps.video_ids, old_id, "canvas_node.ref_id")?;
            if let Some(file_path) = maps
                .video_file_paths
                .get(&old_id)
                .and_then(|value| value.as_ref())
            {
                set_video_studio_json_string(&mut config_json, "file_path", file_path);
            }
            if let Some(thumbnail_path) = maps
                .video_thumbnail_paths
                .get(&old_id)
                .and_then(|value| value.as_ref())
            {
                set_video_studio_json_string(&mut config_json, "thumbnail_path", thumbnail_path);
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
        "legacy_chapter_video_prompt" => Err(VideoStudioProjectImportError::InvalidPackage(
            "视频画布项目包不能导入 legacy_chapter_video_prompt 节点引用".to_string(),
        )),
        other => Err(VideoStudioProjectImportError::InvalidPackage(format!(
            "未知画布节点引用类型: {other}"
        ))),
    }
}

fn remap_node_ref(
    ref_type: Option<String>,
    ref_id: Option<i64>,
    config_json: Value,
    map: &std::collections::HashMap<i64, i64>,
) -> Result<(Option<String>, Option<i64>, Value), VideoStudioProjectImportError> {
    let kind = ref_type.as_deref().unwrap_or_default();
    let old_id = required_video_studio_ref_id(ref_id, kind)?;
    let new_id = remap_video_studio_required_id(map, old_id, "canvas_node.ref_id")?;
    Ok((ref_type, Some(new_id), config_json))
}

fn remap_video_studio_canvas_node_config_ids(
    config_json: &mut Value,
    maps: &VideoStudioProjectImportIdMaps,
) -> Result<(), VideoStudioProjectImportError> {
    let Value::Object(object) = config_json else {
        return Ok(());
    };
    remap_video_studio_json_i64_field(object, "canvas_text_asset_id", &maps.text_asset_ids)?;
    remap_video_studio_json_i64_field(object, "generation_spec_id", &maps.generation_spec_ids)?;
    remap_video_studio_json_i64_field(object, "prompt_artifact_id", &maps.prompt_artifact_ids)?;
    remap_video_studio_json_i64_field(object, "generated_image_id", &maps.image_ids)?;
    remap_video_studio_json_i64_field(object, "legacy_generated_image_id", &maps.image_ids)?;
    remap_video_studio_json_i64_field(object, "source_image_id", &maps.image_ids)?;
    remap_video_studio_json_i64_field(object, "generated_video_id", &maps.video_ids)?;
    remap_video_studio_json_i64_field(object, "legacy_generated_video_id", &maps.video_ids)?;
    remap_video_studio_json_i64_field(object, "first_frame_image_id", &maps.image_ids)?;
    remap_video_studio_json_i64_array_field(object, "reference_image_ids", &maps.image_ids)?;
    remap_video_studio_json_i64_array_field(object, "video_ids", &maps.video_ids)?;
    remap_video_studio_json_i64_field(object, "video_id", &maps.video_ids)?;
    remap_video_studio_json_i64_field(object, "source_video_id", &maps.video_ids)?;
    remap_video_studio_json_i64_field(object, "media_asset_id", &maps.media_asset_ids)?;
    remap_video_studio_json_i64_array_field(object, "media_asset_ids", &maps.media_asset_ids)?;
    remap_video_studio_json_i64_field(object, "preview_media_asset_id", &maps.media_asset_ids)?;
    remap_video_studio_json_i64_field(object, "video_segment_id", &maps.video_segment_ids)?;
    remap_video_studio_json_i64_array_field(object, "video_segment_ids", &maps.video_segment_ids)?;
    remap_video_studio_json_i64_field(object, "source_segment_id", &maps.video_segment_ids)?;
    remap_video_studio_json_i64_field(object, "assembly_timeline_id", &maps.assembly_timeline_ids)?;
    remap_video_studio_json_i64_array_field(object, "assembly_clip_ids", &maps.assembly_clip_ids)?;
    Ok(())
}

fn set_video_studio_json_string(config_json: &mut Value, key: &str, value: &str) {
    if let Value::Object(object) = config_json {
        object.insert(key.to_string(), Value::String(value.to_string()));
    }
}

fn required_video_studio_ref_id(
    ref_id: Option<i64>,
    ref_type: &str,
) -> Result<i64, VideoStudioProjectImportError> {
    let ref_id = ref_id.ok_or_else(|| {
        VideoStudioProjectImportError::InvalidPackage(format!("{ref_type} 画布节点必须设置 ref_id"))
    })?;
    if ref_id <= 0 {
        return Err(VideoStudioProjectImportError::InvalidPackage(format!(
            "{ref_type} 画布节点 ref_id 必须是正整数"
        )));
    }
    Ok(ref_id)
}
