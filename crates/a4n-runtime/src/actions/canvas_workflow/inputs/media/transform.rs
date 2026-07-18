use super::super::rules::{canvas_config_f32, canvas_config_string, incoming_edges};
use super::super::sources::source_generated_video_id;
use crate::state::AppState;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;

pub(in crate::actions::canvas_workflow) async fn build_canvas_trim_input_for_run(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<(i64, f32, f32, Vec<i64>), String> {
    let incoming = incoming_edges(document, node.id);
    let mut video_id = None;
    let mut source_node_ids = Vec::new();
    for edge in incoming {
        let source = document
            .nodes
            .iter()
            .find(|candidate| candidate.id == edge.source_node_id)
            .ok_or_else(|| format!("画布连线引用不存在的源节点: {}", edge.source_node_id))?;
        source_node_ids.push(source.id);
        if let Some(id) =
            source_generated_video_id(state, workflow_run_id, source, "TrimVideoNode").await?
        {
            video_id = Some(id);
        }
    }
    let video_id = video_id.ok_or_else(|| "TrimVideoNode 缺少源视频".to_string())?;
    let start_secs = canvas_config_f32(&node.config_json, "start_secs").unwrap_or(0.0);
    let end_secs = canvas_config_f32(&node.config_json, "end_secs")
        .ok_or_else(|| "TrimVideoNode 缺少 end_secs".to_string())?;
    Ok((video_id, start_secs, end_secs, source_node_ids))
}

pub(in crate::actions::canvas_workflow) async fn build_canvas_extract_frame_input_for_run(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<(i64, String, Option<f32>, Vec<i64>), String> {
    let incoming = incoming_edges(document, node.id);
    let mut video_id = None;
    let mut source_node_ids = Vec::new();
    for edge in incoming {
        let source = document
            .nodes
            .iter()
            .find(|candidate| candidate.id == edge.source_node_id)
            .ok_or_else(|| format!("画布连线引用不存在的源节点: {}", edge.source_node_id))?;
        source_node_ids.push(source.id);
        if let Some(id) =
            source_generated_video_id(state, workflow_run_id, source, "ExtractFrameNode").await?
        {
            video_id = Some(id);
        }
    }
    let video_id = video_id.ok_or_else(|| "ExtractFrameNode 缺少源视频".to_string())?;
    let frame_kind = canvas_config_string(&node.config_json, &["frame_kind", "kind"])
        .unwrap_or_else(|| "first".to_string());
    let at_secs = canvas_config_f32(&node.config_json, "at_secs");
    Ok((video_id, frame_kind, at_secs, source_node_ids))
}
