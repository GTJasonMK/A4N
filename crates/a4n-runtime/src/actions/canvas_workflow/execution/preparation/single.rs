use super::super::super::{RunCanvasNodeInput, WORKFLOW_KIND_VIDEO_CANVAS_GRAPH};
use super::super::contracts::PreparedCanvasGraphRun;
use super::super::graph::{is_runnable_canvas_node, unsupported_canvas_node_execution_error};
use super::context::load_canvas_execution_document;
use crate::AppState;
use a4n_orchestrator::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use chrono::Utc;
use serde_json::json;

pub(in crate::actions::canvas_workflow::execution) async fn prepare_canvas_single_node_run(
    state: &AppState,
    user_id: i64,
    input: RunCanvasNodeInput,
) -> Result<PreparedCanvasGraphRun, String> {
    let document =
        load_canvas_execution_document(state, user_id, input.project_id, input.graph_id).await?;
    let node = document
        .nodes
        .iter()
        .find(|node| node.id == input.node_id)
        .ok_or_else(|| format!("画布节点不存在: {}", input.node_id))?;
    if !is_runnable_canvas_node(node) {
        return Err(unsupported_canvas_node_execution_error(&node.node_kind));
    }
    let step_key = format!("canvas_node:{}", input.node_id);
    let graph_input_json = json!({
        "project_id": input.project_id,
        "graph_id": input.graph_id,
        "graph_kind": document.graph.graph_kind,
        "graph_version": document.graph.version,
        "node_id": input.node_id,
        "mode": "single_node",
        "runnable_node_ids": [input.node_id],
    });
    let graph_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!(
                "video_canvas_graph:{}:single_node:{}:{}",
                input.graph_id,
                input.node_id,
                Utc::now().timestamp_micros()
            ),
            workflow_kind: WORKFLOW_KIND_VIDEO_CANVAS_GRAPH.to_string(),
            project_id: Some(input.project_id),
            root_project_id: Some(input.project_id),
            source_snapshot_id: None,
            scope_table: Some("canvas_graph".to_string()),
            scope_id: Some(input.graph_id),
            chapter_id: None,
            mode: Some("single_node".to_string()),
            input_fingerprint_json: Some(graph_input_json),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(step_key),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    Ok(PreparedCanvasGraphRun {
        workflow_run_id: graph_run.id,
        project_id: input.project_id,
        graph_id: input.graph_id,
        mode: "single_node".to_string(),
        runnable_node_ids: vec![input.node_id],
    })
}
