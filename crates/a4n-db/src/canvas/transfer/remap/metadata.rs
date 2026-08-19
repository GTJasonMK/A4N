use super::super::types::{VideoStudioProjectImportError, VideoStudioProjectImportIdMaps};
use super::ids::{
    remap_video_studio_json_i64_array_field, remap_video_studio_json_i64_field,
    remap_video_studio_required_id,
};
use a4n_domain::{
    GeneratedMediaReference, canvas_cache_key, clear_canvas_transfer_runtime_identity,
    parse_canvas_cache_key, parse_generated_media_reference,
};
use serde_json::Value;

pub(in crate::canvas::transfer) fn remap_video_studio_canvas_metadata(
    mut metadata: Value,
    maps: &VideoStudioProjectImportIdMaps,
) -> Result<Value, VideoStudioProjectImportError> {
    remap_video_studio_canvas_metadata_object(&mut metadata, maps)?;
    clear_canvas_transfer_runtime_identity(&mut metadata);
    Ok(metadata)
}

fn remap_video_studio_canvas_metadata_object(
    value: &mut Value,
    maps: &VideoStudioProjectImportIdMaps,
) -> Result<(), VideoStudioProjectImportError> {
    let Some(object) = value.as_object_mut() else {
        return Ok(());
    };
    remap_video_studio_json_i64_field(object, "generation_spec_id", &maps.generation_spec_ids)?;
    remap_video_studio_json_i64_field(object, "prompt_artifact_id", &maps.prompt_artifact_ids)?;
    remap_video_studio_json_i64_field(object, "canvas_text_asset_id", &maps.text_asset_ids)?;
    remap_video_studio_json_i64_field(object, "generated_image_id", &maps.image_ids)?;
    remap_video_studio_json_i64_field(object, "generated_video_id", &maps.video_ids)?;
    remap_video_studio_json_i64_field(object, "image_id", &maps.image_ids)?;
    remap_video_studio_json_i64_field(object, "video_id", &maps.video_ids)?;
    remap_video_studio_json_i64_array_field(object, "video_ids", &maps.video_ids)?;
    remap_video_studio_json_i64_field(object, "source_video_id", &maps.video_ids)?;
    remap_video_studio_json_i64_field(object, "source_image_id", &maps.image_ids)?;
    remap_video_studio_json_i64_field(object, "first_frame_image_id", &maps.image_ids)?;
    remap_video_studio_json_i64_field(object, "last_frame_image_id", &maps.image_ids)?;
    remap_video_studio_json_i64_array_field(object, "reference_image_ids", &maps.image_ids)?;
    remap_video_studio_json_i64_field(object, "graph_id", &maps.graph_ids)?;
    remap_video_studio_json_i64_field(object, "canvas_graph_id", &maps.graph_ids)?;
    remap_video_studio_json_i64_field(object, "node_id", &maps.node_ids)?;
    remap_video_studio_json_i64_field(object, "canvas_node_id", &maps.node_ids)?;
    remap_video_studio_json_i64_array_field(object, "source_node_ids", &maps.node_ids)?;
    remap_media_fields(object, maps)?;
    for key in [
        "canvas",
        "source_material",
        "params",
        "node_config",
        "metadata",
    ] {
        if let Some(nested) = object.get_mut(key) {
            remap_video_studio_canvas_metadata_object(nested, maps)?;
        }
    }
    Ok(())
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
    let Some(reference) = parse_generated_media_reference(raw).ok().flatten() else {
        return Ok(Some(raw.to_string()));
    };
    let ids = match reference {
        GeneratedMediaReference::Video(_) => &maps.video_ids,
        GeneratedMediaReference::Image(_) => &maps.image_ids,
    };
    let new_id = remap_video_studio_required_id(
        ids,
        reference.id(),
        "media_provenance.source_external_path",
    )?;
    Ok(Some(reference.with_id(new_id).to_string()))
}

pub(in crate::canvas::transfer) fn remap_video_studio_canvas_cache_key(
    value: Option<&str>,
    maps: &VideoStudioProjectImportIdMaps,
) -> Result<Option<String>, VideoStudioProjectImportError> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let Some((graph_id, node_id)) = parse_canvas_cache_key(raw) else {
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
    Ok(Some(canvas_cache_key(new_graph_id, new_node_id)))
}
