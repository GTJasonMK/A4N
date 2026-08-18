use std::collections::HashSet;

use serde_json::Value;

use super::refs::{validate_snapshot_i64_array_ref, validate_snapshot_i64_ref};

use a4n_core::models::media::media_provenance;

pub(in crate::project::video_studio_package::validation) struct MediaProvenanceSnapshotRefs<'a> {
    pub(in crate::project::video_studio_package::validation) graph_ids: &'a HashSet<i64>,
    pub(in crate::project::video_studio_package::validation) node_ids: &'a HashSet<i64>,
    pub(in crate::project::video_studio_package::validation) text_asset_ids: &'a HashSet<i64>,
    pub(in crate::project::video_studio_package::validation) generation_spec_ids: &'a HashSet<i64>,
    pub(in crate::project::video_studio_package::validation) prompt_artifact_ids: &'a HashSet<i64>,
    pub(in crate::project::video_studio_package::validation) image_ids: &'a HashSet<i64>,
    pub(in crate::project::video_studio_package::validation) video_ids: &'a HashSet<i64>,
    pub(in crate::project::video_studio_package::validation) media_asset_ids: &'a HashSet<i64>,
    pub(in crate::project::video_studio_package::validation) video_segment_ids: &'a HashSet<i64>,
    pub(in crate::project::video_studio_package::validation) assembly_timeline_ids:
        &'a HashSet<i64>,
    pub(in crate::project::video_studio_package::validation) assembly_clip_ids: &'a HashSet<i64>,
}

pub(in crate::project::video_studio_package::validation) fn validate_media_provenance_snapshot(
    provenance: &media_provenance::Model,
    refs: &MediaProvenanceSnapshotRefs<'_>,
) -> Result<(), String> {
    let Some(snapshot) = provenance.request_snapshot_json.as_ref() else {
        return Ok(());
    };
    let Some(object) = snapshot.as_object() else {
        return Err(format!(
            "媒体来源 request_snapshot_json 必须是 JSON 对象: media_asset_id={}",
            provenance.media_asset_id
        ));
    };
    validate_snapshot_object_refs(provenance.media_asset_id, object, refs)?;
    if let Some(canvas) = object.get("canvas") {
        let Some(canvas_object) = canvas.as_object() else {
            return Err(format!(
                "媒体来源 request_snapshot_json.canvas 必须是 JSON 对象: media_asset_id={}",
                provenance.media_asset_id
            ));
        };
        validate_snapshot_i64_ref(
            provenance.media_asset_id,
            canvas_object,
            "canvas.graph_id",
            "graph_id",
            refs.graph_ids,
            "画布图",
        )?;
        validate_snapshot_i64_ref(
            provenance.media_asset_id,
            canvas_object,
            "canvas.node_id",
            "node_id",
            refs.node_ids,
            "画布节点",
        )?;
        validate_snapshot_object_refs(provenance.media_asset_id, canvas_object, refs)?;
    }
    Ok(())
}

fn validate_snapshot_object_refs(
    media_asset_id: i64,
    object: &serde_json::Map<String, Value>,
    refs: &MediaProvenanceSnapshotRefs<'_>,
) -> Result<(), String> {
    validate_snapshot_i64_ref(
        media_asset_id,
        object,
        "generation_spec_id",
        "generation_spec_id",
        refs.generation_spec_ids,
        "生成意图",
    )?;
    validate_snapshot_i64_ref(
        media_asset_id,
        object,
        "prompt_artifact_id",
        "prompt_artifact_id",
        refs.prompt_artifact_ids,
        "提示词产物",
    )?;
    validate_snapshot_i64_ref(
        media_asset_id,
        object,
        "canvas_text_asset_id",
        "canvas_text_asset_id",
        refs.text_asset_ids,
        "画布文本资产",
    )?;
    for key in ["generated_image_id", "image_id", "source_image_id"] {
        validate_snapshot_i64_ref(media_asset_id, object, key, key, refs.image_ids, "生成图片")?;
    }
    for key in ["generated_video_id", "video_id", "source_video_id"] {
        validate_snapshot_i64_ref(media_asset_id, object, key, key, refs.video_ids, "生成视频")?;
    }
    validate_snapshot_i64_ref(
        media_asset_id,
        object,
        "media_asset_id",
        "media_asset_id",
        refs.media_asset_ids,
        "媒体资产",
    )?;
    validate_snapshot_i64_array_ref(
        media_asset_id,
        object,
        "media_asset_ids",
        "media_asset_ids",
        refs.media_asset_ids,
        "媒体资产",
    )?;
    validate_snapshot_i64_ref(
        media_asset_id,
        object,
        "preview_media_asset_id",
        "preview_media_asset_id",
        refs.media_asset_ids,
        "媒体资产",
    )?;
    validate_snapshot_i64_ref(
        media_asset_id,
        object,
        "video_segment_id",
        "video_segment_id",
        refs.video_segment_ids,
        "视频片段",
    )?;
    validate_snapshot_i64_array_ref(
        media_asset_id,
        object,
        "video_segment_ids",
        "video_segment_ids",
        refs.video_segment_ids,
        "视频片段",
    )?;
    validate_snapshot_i64_ref(
        media_asset_id,
        object,
        "source_segment_id",
        "source_segment_id",
        refs.video_segment_ids,
        "视频片段",
    )?;
    validate_snapshot_i64_ref(
        media_asset_id,
        object,
        "assembly_timeline_id",
        "assembly_timeline_id",
        refs.assembly_timeline_ids,
        "成片时间线",
    )?;
    validate_snapshot_i64_array_ref(
        media_asset_id,
        object,
        "assembly_clip_ids",
        "assembly_clip_ids",
        refs.assembly_clip_ids,
        "成片剪辑",
    )?;
    Ok(())
}
