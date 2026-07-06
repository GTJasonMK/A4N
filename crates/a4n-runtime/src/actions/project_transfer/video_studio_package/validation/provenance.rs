use super::*;
use serde_json::Value;

pub(super) fn validate_media_provenance_source_external_path(
    provenance: &media_provenance::Model,
    image_ids: &HashSet<i64>,
    video_ids: &HashSet<i64>,
) -> Result<(), String> {
    let Some(raw) = provenance.source_external_path.as_deref() else {
        return Ok(());
    };
    if let Some(suffix) = raw.strip_prefix("generated_video:") {
        let video_id = suffix.parse::<i64>().map_err(|_| {
            format!(
                "媒体来源 source_external_path 的 generated_video ID 无效: media_asset_id={}, source_external_path={raw}",
                provenance.media_asset_id
            )
        })?;
        if !video_ids.contains(&video_id) {
            return Err(format!(
                "媒体来源 source_external_path 引用了不存在的生成视频: media_asset_id={}, generated_video_id={video_id}",
                provenance.media_asset_id
            ));
        }
    } else if let Some(suffix) = raw.strip_prefix("generated_image:") {
        let image_id = suffix.parse::<i64>().map_err(|_| {
            format!(
                "媒体来源 source_external_path 的 generated_image ID 无效: media_asset_id={}, source_external_path={raw}",
                provenance.media_asset_id
            )
        })?;
        if !image_ids.contains(&image_id) {
            return Err(format!(
                "媒体来源 source_external_path 引用了不存在的生成图片: media_asset_id={}, generated_image_id={image_id}",
                provenance.media_asset_id
            ));
        }
    }
    Ok(())
}

pub(super) struct MediaProvenanceSnapshotRefs<'a> {
    pub(super) graph_ids: &'a HashSet<i64>,
    pub(super) node_ids: &'a HashSet<i64>,
    pub(super) text_asset_ids: &'a HashSet<i64>,
    pub(super) generation_spec_ids: &'a HashSet<i64>,
    pub(super) prompt_artifact_ids: &'a HashSet<i64>,
    pub(super) image_ids: &'a HashSet<i64>,
    pub(super) video_ids: &'a HashSet<i64>,
    pub(super) media_asset_ids: &'a HashSet<i64>,
    pub(super) video_segment_ids: &'a HashSet<i64>,
    pub(super) assembly_timeline_ids: &'a HashSet<i64>,
    pub(super) assembly_clip_ids: &'a HashSet<i64>,
}

pub(super) fn validate_media_provenance_snapshot(
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

fn validate_snapshot_i64_ref(
    media_asset_id: i64,
    object: &serde_json::Map<String, Value>,
    label: &str,
    key: &str,
    ids: &HashSet<i64>,
    target_label: &str,
) -> Result<(), String> {
    let Some(value) = object.get(key) else {
        return Ok(());
    };
    let id = value.as_i64().ok_or_else(|| {
        format!(
            "媒体来源 request_snapshot_json.{label} 必须是整数 ID: media_asset_id={media_asset_id}"
        )
    })?;
    if !ids.contains(&id) {
        return Err(format!(
            "媒体来源 request_snapshot_json.{label} 引用了不存在的{target_label}: media_asset_id={media_asset_id}, id={id}"
        ));
    }
    Ok(())
}

fn validate_snapshot_i64_array_ref(
    media_asset_id: i64,
    object: &serde_json::Map<String, Value>,
    label: &str,
    key: &str,
    ids: &HashSet<i64>,
    target_label: &str,
) -> Result<(), String> {
    let Some(value) = object.get(key) else {
        return Ok(());
    };
    let values = value.as_array().ok_or_else(|| {
        format!(
            "媒体来源 request_snapshot_json.{label} 必须是整数 ID 数组: media_asset_id={media_asset_id}"
        )
    })?;
    for (index, value) in values.iter().enumerate() {
        let id = value.as_i64().ok_or_else(|| {
            format!(
                "媒体来源 request_snapshot_json.{label}[{index}] 必须是整数 ID: media_asset_id={media_asset_id}"
            )
        })?;
        if !ids.contains(&id) {
            return Err(format!(
                "媒体来源 request_snapshot_json.{label}[{index}] 引用了不存在的{target_label}: media_asset_id={media_asset_id}, id={id}"
            ));
        }
    }
    Ok(())
}
