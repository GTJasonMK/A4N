use super::super::contracts::CanvasExportVideoInput;
use super::super::rules::incoming_edges;
use super::super::sources::source_generated_video_id;
use crate::state::AppState;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;

pub(in crate::actions::canvas_workflow) async fn build_canvas_export_video_input_for_run(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<CanvasExportVideoInput, String> {
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
        if edge.target_port == "video"
            && let Some(id) =
                source_generated_video_id(state, workflow_run_id, source, "VideoOutputNode").await?
        {
            video_id = Some(id);
        }
    }

    if video_id.is_none() {
        return Err("VideoOutputNode 缺少可输出的视频输入".to_string());
    }
    Ok(CanvasExportVideoInput {
        video_id,
        source_node_ids,
    })
}
