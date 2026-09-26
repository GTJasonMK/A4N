use a4n_db::canvas::CanvasGraphRecordDocument;
use a4n_db_models::models::{canvas::canvas_node, generation::workflow_checkpoint};
use a4n_domain::{CanvasFrameKind, CanvasNodeRefKind};
use serde_json::Value;

use crate::canvas::{
    canvas_config_string, canvas_edge_order_index, first_canvas_text_asset_id, incoming_edges,
    source_checkpoint::source_node_output_id, source_node, validate_canvas_clip_order,
};

use super::contracts::{
    CanvasAssembleVideoNodeInput, CanvasEditingNodeInput, CanvasExtractFrameNodeInput,
    CanvasTrimVideoNodeInput,
};

#[cfg(test)]
pub(crate) fn build_canvas_editing_node_input_for_preview(
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
) -> Result<CanvasEditingNodeInput, String> {
    build_canvas_editing_node_input(node, document, None)
}

pub(crate) fn build_canvas_editing_node_input(
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
    checkpoints: Option<&[workflow_checkpoint::Model]>,
) -> Result<CanvasEditingNodeInput, String> {
    match node.node_kind.as_str() {
        "ExtractFrameNode" => build_extract_frame_input(node, document, checkpoints)
            .map(CanvasEditingNodeInput::ExtractFrame),
        "TrimVideoNode" => build_trim_video_input(node, document, checkpoints)
            .map(CanvasEditingNodeInput::TrimVideo),
        "AssembleVideoNode" => build_assemble_video_input(node, document, checkpoints)
            .map(CanvasEditingNodeInput::AssembleVideo),
        other => Err(format!("不支持的 Canvas 编辑节点类型: {other}")),
    }
}

fn build_extract_frame_input(
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
    checkpoints: Option<&[workflow_checkpoint::Model]>,
) -> Result<CanvasExtractFrameNodeInput, String> {
    let (video_id, source_node_ids) =
        resolve_single_source_video(node, document, checkpoints, "ExtractFrameNode")?;
    let frame_kind = node
        .config_json
        .get("frame_kind")
        .and_then(Value::as_str)
        .ok_or_else(|| "ExtractFrameNode 缺少字符串 frame_kind".to_string())?
        .parse::<CanvasFrameKind>()
        .map_err(|error| error.to_string())?;
    let at_secs = strict_canvas_config_f32(&node.config_json, "at_secs")?;
    Ok(CanvasExtractFrameNodeInput {
        video_id,
        frame_kind,
        at_secs,
        canvas_text_asset_id: first_canvas_text_asset_id(node, document)?,
        source_node_ids,
    })
}

fn build_trim_video_input(
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
    checkpoints: Option<&[workflow_checkpoint::Model]>,
) -> Result<CanvasTrimVideoNodeInput, String> {
    let (video_id, source_node_ids) =
        resolve_single_source_video(node, document, checkpoints, "TrimVideoNode")?;
    let start_secs = strict_canvas_config_f32(&node.config_json, "start_secs")?
        .ok_or_else(|| "TrimVideoNode 缺少 start_secs".to_string())?;
    let end_secs = strict_canvas_config_f32(&node.config_json, "end_secs")?
        .ok_or_else(|| "TrimVideoNode 缺少 end_secs".to_string())?;
    Ok(CanvasTrimVideoNodeInput {
        video_id,
        start_secs,
        end_secs,
        canvas_text_asset_id: first_canvas_text_asset_id(node, document)?,
        source_node_ids,
    })
}

fn strict_canvas_config_f32(config: &Value, key: &str) -> Result<Option<f32>, String> {
    config
        .get(key)
        .map(|value| {
            let value = value
                .as_f64()
                .ok_or_else(|| format!("{key} 必须是有限 f32 数字"))?;
            if !value.is_finite() || value < f32::MIN as f64 || value > f32::MAX as f64 {
                return Err(format!("{key} 必须是有限 f32 数字"));
            }
            Ok(value as f32)
        })
        .transpose()
}

fn build_assemble_video_input(
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
    checkpoints: Option<&[workflow_checkpoint::Model]>,
) -> Result<CanvasAssembleVideoNodeInput, String> {
    if node.config_json.get("video_ids").is_some() {
        return Err(
            "AssembleVideoNode 不支持 config_json.video_ids，请通过 VideoInputNode 连线提供视频片段"
                .to_string(),
        );
    }
    let mut clips = Vec::<(i64, i64, i64)>::new();
    for edge in incoming_edges(document, node.id) {
        let source = source_node(document, edge.source_node_id)?;
        if let Some(video_id) = source_node_output_id(
            checkpoints,
            source,
            "AssembleVideoNode",
            "generated_video_id",
            CanvasNodeRefKind::GeneratedVideo,
        )? {
            clips.push((canvas_edge_order_index(edge)?, video_id, source.id));
        }
    }
    if clips.is_empty() {
        return Err("AssembleVideoNode 缺少视频片段".to_string());
    }
    clips.sort_by_key(|(order_index, _, _)| *order_index);
    validate_canvas_clip_order(&clips)?;
    Ok(CanvasAssembleVideoNodeInput {
        video_ids: clips.iter().map(|(_, video_id, _)| *video_id).collect(),
        title: canvas_config_string(&node.config_json, "title"),
        canvas_text_asset_id: first_canvas_text_asset_id(node, document)?,
        source_node_ids: clips
            .iter()
            .map(|(_, _, source_node_id)| *source_node_id)
            .collect(),
    })
}

fn resolve_single_source_video(
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
    checkpoints: Option<&[workflow_checkpoint::Model]>,
    consumer_label: &str,
) -> Result<(i64, Vec<i64>), String> {
    let mut video_id = None;
    let mut source_node_ids = Vec::new();
    for edge in incoming_edges(document, node.id) {
        let source = source_node(document, edge.source_node_id)?;
        source_node_ids.push(source.id);
        if let Some(id) = source_node_output_id(
            checkpoints,
            source,
            consumer_label,
            "generated_video_id",
            CanvasNodeRefKind::GeneratedVideo,
        )? {
            video_id = Some(id);
        }
    }
    let video_id = video_id.ok_or_else(|| format!("{consumer_label} 缺少源视频"))?;
    Ok((video_id, source_node_ids))
}
