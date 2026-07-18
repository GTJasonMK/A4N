use super::super::super::{RunCanvasGraphInput, WORKFLOW_KIND_VIDEO_CANVAS_GRAPH};
use super::super::contracts::PreparedCanvasGraphRun;
use super::super::graph::runnable_canvas_node_ids;
use super::context::{load_canvas_execution_document, validate_canvas_output_target};
use crate::AppState;
use a4n_orchestrator::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use chrono::Utc;
use serde_json::json;

pub(in crate::actions::canvas_workflow::execution) async fn prepare_canvas_graph_run(
    state: &AppState,
    user_id: i64,
    input: RunCanvasGraphInput,
) -> Result<PreparedCanvasGraphRun, String> {
    let document =
        load_canvas_execution_document(state, user_id, input.project_id, input.graph_id).await?;
    validate_canvas_output_target(&document, input.target_node_id)?;
    let mode = if input.target_node_id.is_some() {
        "run_to_node"
    } else {
        "full_graph"
    };
    let runnable_node_ids = runnable_canvas_node_ids(&document, input.target_node_id)?;
    if runnable_node_ids.is_empty() {
        return Err("画布中没有可执行的操作节点".to_string());
    }
    let graph_input_json = json!({
        "project_id": input.project_id,
        "graph_id": input.graph_id,
        "graph_kind": document.graph.graph_kind,
        "graph_version": document.graph.version,
        "target_node_id": input.target_node_id,
        "runnable_node_ids": runnable_node_ids.clone(),
    });
    let first_step_key = runnable_node_ids
        .first()
        .map(|node_id| format!("canvas_node:{node_id}"));
    let graph_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!(
                "video_canvas_graph:{}:{mode}:graph:{}",
                input.graph_id,
                Utc::now().timestamp_micros()
            ),
            workflow_kind: WORKFLOW_KIND_VIDEO_CANVAS_GRAPH.to_string(),
            project_id: Some(input.project_id),
            root_project_id: Some(input.project_id),
            source_snapshot_id: None,
            scope_table: Some("canvas_graph".to_string()),
            scope_id: Some(input.graph_id),
            chapter_id: None,
            mode: Some(mode.to_string()),
            input_fingerprint_json: Some(graph_input_json),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: first_step_key,
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
        mode: mode.to_string(),
        runnable_node_ids,
    })
}
