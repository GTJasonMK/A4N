use super::super::super::WORKFLOW_KIND_VIDEO_CANVAS_GRAPH;
use super::super::contracts::PreparedCanvasGraphRun;
use super::super::graph::{
    is_runnable_canvas_node, runnable_canvas_node_ids, unsupported_canvas_node_execution_error,
};
use super::context::{load_canvas_execution_document, validate_canvas_output_target};
use crate::AppState;
use a4n_orchestrator::workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_RUNNING};
use serde_json::Value;
use std::collections::BTreeSet;

pub(in crate::actions::canvas_workflow::execution) async fn prepare_canvas_graph_resume(
    state: &AppState,
    user_id: i64,
    workflow_run_id: i64,
) -> Result<PreparedCanvasGraphRun, String> {
    let run = state
        .orchestrator
        .workflow_run_model_by_id(workflow_run_id)
        .await
        .map_err(|err| err.to_string())?;
    if run.workflow_kind != WORKFLOW_KIND_VIDEO_CANVAS_GRAPH {
        return Err(format!(
            "工作流类型 {} 不能按画布运行恢复",
            run.workflow_kind
        ));
    }
    let project_id = run
        .project_id
        .ok_or_else(|| "画布运行缺少 project_id".to_string())?;
    let graph_id = run
        .scope_id
        .filter(|_| run.scope_table.as_deref() == Some("canvas_graph"))
        .ok_or_else(|| "画布运行缺少 canvas_graph scope".to_string())?;

    let target_node_id = run
        .input_fingerprint_json
        .as_ref()
        .and_then(|value| value.get("target_node_id"))
        .and_then(Value::as_i64);
    let mode = run.mode.unwrap_or_else(|| {
        if target_node_id.is_some() {
            "run_to_node".to_string()
        } else {
            "full_graph".to_string()
        }
    });
    let single_node_id = if mode == "single_node" {
        Some(
            run.input_fingerprint_json
                .as_ref()
                .and_then(|value| value.get("node_id"))
                .and_then(Value::as_i64)
                .ok_or_else(|| "单节点运行缺少 node_id".to_string())?,
        )
    } else {
        None
    };
    let document = load_canvas_execution_document(state, user_id, project_id, graph_id).await?;
    validate_canvas_output_target(&document, target_node_id)?;
    if let Some(single_node_id) = single_node_id {
        let node = document
            .nodes
            .iter()
            .find(|node| node.id == single_node_id)
            .ok_or_else(|| format!("画布节点不存在: {single_node_id}"))?;
        if !is_runnable_canvas_node(node) {
            return Err(unsupported_canvas_node_execution_error(&node.node_kind));
        }
    }
    let runnable_node_ids = match single_node_id {
        Some(node_id) => vec![node_id],
        None => runnable_canvas_node_ids(&document, target_node_id)?,
    };
    let completed_step_keys = state
        .orchestrator
        .workflow_checkpoint_models(workflow_run_id)
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .filter(|checkpoint| checkpoint.status == WORKFLOW_STATUS_COMPLETED)
        .map(|checkpoint| checkpoint.step_key)
        .collect::<BTreeSet<_>>();
    let pending_node_ids = runnable_node_ids
        .into_iter()
        .filter(|node_id| !completed_step_keys.contains(&format!("canvas_node:{node_id}")))
        .collect::<Vec<_>>();
    if pending_node_ids.is_empty() {
        let completed = state
            .orchestrator
            .mark_workflow_status(workflow_run_id, WORKFLOW_STATUS_COMPLETED, false, None)
            .await
            .map_err(|err| err.to_string())?;
        return Ok(PreparedCanvasGraphRun {
            workflow_run_id: completed.id,
            project_id,
            graph_id,
            mode,
            runnable_node_ids: Vec::new(),
        });
    }
    let first_step_key = pending_node_ids
        .first()
        .map(|node_id| format!("canvas_node:{node_id}"));
    state
        .orchestrator
        .mark_workflow_status(workflow_run_id, WORKFLOW_STATUS_RUNNING, true, None)
        .await
        .map_err(|err| err.to_string())?;
    if let Some(first_step_key) = first_step_key {
        state
            .orchestrator
            .mark_workflow_current_step(workflow_run_id, &first_step_key)
            .await
            .map_err(|err| err.to_string())?;
    }
    Ok(PreparedCanvasGraphRun {
        workflow_run_id,
        project_id,
        graph_id,
        mode,
        runnable_node_ids: pending_node_ids,
    })
}
