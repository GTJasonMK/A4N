use super::super::execution::record_canvas_node_checkpoint;
use super::super::inputs::build_canvas_output_value;
use super::super::node_lifecycle::{
    CanvasRunHandle, finish_canvas_node_success, start_canvas_node_run,
};
use super::super::persistence::update_canvas_node_observation;
use super::super::{CanvasGraphDocument, CanvasRunSummary};
use super::artifacts::canvas_probe_artifact_json;
use super::contracts::{canvas_probe_observation_patch, probe_node_data_type};
use crate::AppState;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::workflow::WORKFLOW_STATUS_COMPLETED;
use serde_json::json;

pub(in crate::actions::canvas_workflow) async fn run_canvas_probe_node(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let data_type = probe_node_data_type(&node.node_kind)?;
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let output = build_canvas_output_value(
        Some(state),
        upstream_run_id,
        node,
        document,
        data_type,
        &node.node_kind,
    )
    .await?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "data_type": data_type,
        "source_node_ids": output.source_node_ids});
    let run = start_canvas_node_run(
        state,
        project_id,
        graph_id,
        mode,
        &step_key,
        input_json.clone(),
        run_handle,
    )
    .await?;
    let artifact_json = canvas_probe_artifact_json(state, data_type, &output).await?;
    update_canvas_node_observation(
        state,
        node.id,
        canvas_probe_observation_patch(&artifact_json, &output.source_node_ids),
    )
    .await?;
    record_canvas_node_checkpoint(
        state,
        run.workflow_run_id,
        step_key,
        WORKFLOW_STATUS_COMPLETED,
        input_json,
        Some(artifact_json),
        None,
        true,
    )
    .await?;
    finish_canvas_node_success(state, run, project_id, graph_id, mode).await
}
