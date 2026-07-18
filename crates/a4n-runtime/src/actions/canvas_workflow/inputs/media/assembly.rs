use super::super::rules::{
    canvas_config_string, canvas_edge_order_index, incoming_edges, validate_canvas_clip_order,
};
use super::super::sources::source_generated_video_id;
use crate::state::AppState;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;

pub(in crate::actions::canvas_workflow) async fn build_canvas_assemble_input_for_run(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<(Vec<i64>, Option<String>, Vec<i64>), String> {
    if node.config_json.get("video_ids").is_some() {
        return Err(
            "AssembleVideoNode 不支持 config_json.video_ids，请通过 VideoInputNode 连线提供视频片段"
                .to_string(),
        );
    }
    let incoming = incoming_edges(document, node.id);
    let mut clips = Vec::<(i64, i64, i64)>::new();
    for edge in incoming {
        let source = document
            .nodes
            .iter()
            .find(|candidate| candidate.id == edge.source_node_id)
            .ok_or_else(|| format!("画布连线引用不存在的源节点: {}", edge.source_node_id))?;
        if let Some(id) =
            source_generated_video_id(state, workflow_run_id, source, "AssembleVideoNode").await?
        {
            clips.push((canvas_edge_order_index(edge)?, id, source.id));
        }
    }
    if clips.is_empty() {
        return Err("AssembleVideoNode 缺少视频片段".to_string());
    }
    clips.sort_by_key(|(order_index, _, _)| *order_index);
    validate_canvas_clip_order(&clips)?;
    let video_ids = clips.iter().map(|(_, video_id, _)| *video_id).collect();
    let source_node_ids = clips
        .iter()
        .map(|(_, _, source_node_id)| *source_node_id)
        .collect();
    let title = canvas_config_string(&node.config_json, &["title", "name"]);
    Ok((video_ids, title, source_node_ids))
}
