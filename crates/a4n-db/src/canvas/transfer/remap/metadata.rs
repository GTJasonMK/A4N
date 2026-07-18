use super::super::types::{VideoStudioProjectImportError, VideoStudioProjectImportIdMaps};
use super::ids::{
    remap_video_studio_json_i64_array_field, remap_video_studio_json_i64_field,
    remap_video_studio_required_id,
};
use serde_json::Value;

pub(in crate::canvas::transfer) fn remap_video_studio_canvas_metadata(
    mut metadata: Value,
    maps: &VideoStudioProjectImportIdMaps,
) -> Result<Value, VideoStudioProjectImportError> {
    if let Some(object) = metadata.as_object_mut() {
        remap_video_studio_json_i64_field(object, "generation_spec_id", &maps.generation_spec_ids)?;
        remap_video_studio_json_i64_field(object, "prompt_artifact_id", &maps.prompt_artifact_ids)?;
        remap_video_studio_json_i64_field(object, "canvas_text_asset_id", &maps.text_asset_ids)?;
        remap_video_studio_json_i64_field(object, "generated_image_id", &maps.image_ids)?;
        remap_video_studio_json_i64_field(object, "generated_video_id", &maps.video_ids)?;
        remap_video_studio_json_i64_field(object, "image_id", &maps.image_ids)?;
        remap_video_studio_json_i64_field(object, "video_id", &maps.video_ids)?;
        remap_video_studio_json_i64_field(object, "source_video_id", &maps.video_ids)?;
        remap_video_studio_json_i64_field(object, "source_image_id", &maps.image_ids)?;
        remap_media_fields(object, maps)?;
    }
    let Some(canvas) = metadata.get_mut("canvas").and_then(Value::as_object_mut) else {
        return Ok(metadata);
    };
    remap_video_studio_json_i64_field(canvas, "graph_id", &maps.graph_ids)?;
    remap_video_studio_json_i64_field(canvas, "node_id", &maps.node_ids)?;
    remap_video_studio_json_i64_field(canvas, "generation_spec_id", &maps.generation_spec_ids)?;
    remap_video_studio_json_i64_field(canvas, "prompt_artifact_id", &maps.prompt_artifact_ids)?;
    remap_video_studio_json_i64_field(canvas, "generated_image_id", &maps.image_ids)?;
    remap_video_studio_json_i64_field(canvas, "generated_video_id", &maps.video_ids)?;
    remap_media_fields(canvas, maps)?;
    Ok(metadata)
}

fn remap_media_fields(
    object: &mut serde_json::Map<String, Value>,
    maps: &VideoStudioProjectImportIdMaps,
) -> Result<(), VideoStudioProjectImportError> {
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

pub(in crate::canvas::transfer) fn remap_video_studio_source_external_path(
    value: Option<&str>,
    maps: &VideoStudioProjectImportIdMaps,
) -> Result<Option<String>, VideoStudioProjectImportError> {
    let Some(raw) = value else {
        return Ok(None);
    };
    if let Some(id) = raw
        .strip_prefix("generated_video:")
        .and_then(|suffix| suffix.parse::<i64>().ok())
    {
        let new_id = remap_video_studio_required_id(
            &maps.video_ids,
            id,
            "media_provenance.source_external_path",
        )?;
        return Ok(Some(format!("generated_video:{new_id}")));
    }
    if let Some(id) = raw
        .strip_prefix("generated_image:")
        .and_then(|suffix| suffix.parse::<i64>().ok())
    {
        let new_id = remap_video_studio_required_id(
            &maps.image_ids,
            id,
            "media_provenance.source_external_path",
        )?;
        return Ok(Some(format!("generated_image:{new_id}")));
    }
    Ok(Some(raw.to_string()))
}

pub(in crate::canvas::transfer) fn remap_video_studio_canvas_cache_key(
    value: Option<&str>,
    maps: &VideoStudioProjectImportIdMaps,
) -> Result<Option<String>, VideoStudioProjectImportError> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let Some((graph_id, node_id)) = parse_video_studio_canvas_cache_key(raw) else {
        return Ok(Some(raw.to_string()));
    };
    let new_graph_id = remap_video_studio_required_id(
        &maps.graph_ids,
        graph_id,
        "assembly_timeline.preview_cache_key graph_id",
    )?;
    let new_node_id = remap_video_studio_required_id(
        &maps.node_ids,
        node_id,
        "assembly_timeline.preview_cache_key node_id",
    )?;
    Ok(Some(format!(
        "canvas_graph:{new_graph_id}:node:{new_node_id}"
    )))
}

fn parse_video_studio_canvas_cache_key(value: &str) -> Option<(i64, i64)> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() == 4
        && parts[0] == "canvas_graph"
        && parts[2] == "node"
        && let (Ok(graph_id), Ok(node_id)) = (parts[1].parse::<i64>(), parts[3].parse::<i64>())
    {
        return Some((graph_id, node_id));
    }
    None
}
