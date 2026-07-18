use super::super::contracts::PreparedCanvasGraphRun;
use super::dispatch::run_canvas_operation_node;
use crate::actions::canvas_workflow::node_lifecycle::{
    CanvasRunHandle, canvas_workflow_run_is_cancelled,
};
use crate::actions::canvas_workflow::{CanvasRunSummary, validate_canvas_document_edge_contract};
use crate::state::AppState;
use a4n_orchestrator::workflow::{
    WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};

pub(in crate::actions::canvas_workflow::execution) async fn execute_prepared_canvas_graph_run_with_runtime_config(
    state: &AppState,
    user_id: i64,
    prepared: PreparedCanvasGraphRun,
    runtime_snapshot: Option<crate::runtime_config::RuntimeConfigSnapshot>,
) -> Result<CanvasRunSummary, String> {
    match runtime_snapshot {
        Some(snapshot) => {
            crate::runtime_config::scope(
                snapshot,
                execute_prepared_canvas_graph_run(state, user_id, prepared),
            )
            .await
        }
        None => execute_prepared_canvas_graph_run(state, user_id, prepared).await,
    }
}

async fn execute_prepared_canvas_graph_run(
    state: &AppState,
    user_id: i64,
    prepared: PreparedCanvasGraphRun,
) -> Result<CanvasRunSummary, String> {
    let shared_run = CanvasRunHandle {
        workflow_run_id: prepared.workflow_run_id,
        finalize_on_success: false,
    };
    let mut last_summary = None;
    for node_id in prepared.runnable_node_ids.iter().copied() {
        if canvas_workflow_run_is_cancelled(state, prepared.workflow_run_id).await? {
            return Ok(cancelled_canvas_graph_summary(&prepared));
        }
        let current_document = state
            .orchestrator
            .get_or_create_canvas_graph_record(
                prepared.project_id,
                Some(prepared.graph_id),
                chrono::Utc::now().naive_utc(),
            )
            .await
            .map_err(crate::actions::app_error_message)?;
        validate_canvas_document_edge_contract(&current_document).map_err(|e| e.to_string())?;
        let node = current_document
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .ok_or_else(|| format!("画布节点不存在: {node_id}"))?;
        let summary = run_canvas_operation_node(
            state,
            user_id,
            prepared.project_id,
            prepared.graph_id,
            node,
            &current_document,
            Some(shared_run),
            &prepared.mode,
        )
        .await?;
        let should_stop = summary.status == WORKFLOW_STATUS_FAILED
            || summary.status == WORKFLOW_STATUS_RUNNING
            || summary.status == WORKFLOW_STATUS_CANCELLED;
        last_summary = Some(summary);
        if should_stop {
            break;
        }
    }

    let Some(summary) = last_summary else {
        return Err("画布中没有可执行的操作节点".to_string());
    };
    if canvas_workflow_run_is_cancelled(state, prepared.workflow_run_id).await? {
        return Ok(cancelled_canvas_graph_summary(&prepared));
    }
    if summary.status == WORKFLOW_STATUS_COMPLETED {
        let completed = state
            .orchestrator
            .mark_workflow_status(
                prepared.workflow_run_id,
                WORKFLOW_STATUS_COMPLETED,
                false,
                None,
            )
            .await
            .map_err(|e| e.to_string())?;
        return Ok(CanvasRunSummary {
            workflow_run_id: completed.id,
            project_id: prepared.project_id,
            graph_id: prepared.graph_id,
            mode: prepared.mode,
            status: completed.status,
            error_message: None,
        });
    }
    Ok(summary)
}

fn cancelled_canvas_graph_summary(prepared: &PreparedCanvasGraphRun) -> CanvasRunSummary {
    CanvasRunSummary {
        workflow_run_id: prepared.workflow_run_id,
        project_id: prepared.project_id,
        graph_id: prepared.graph_id,
        mode: prepared.mode.clone(),
        status: WORKFLOW_STATUS_CANCELLED.to_string(),
        error_message: Some("用户取消画布运行".to_string()),
    }
}
