use super::prelude::*;
use super::*;

pub async fn run_canvas_graph(
    state: &AppState,
    user_id: i64,
    input: RunCanvasGraphInput,
) -> Result<CanvasRunSummary, String> {
    let prepared = prepare_canvas_graph_run(state, user_id, input).await?;
    submit_prepared_canvas_graph_run(state, user_id, prepared)
}

pub async fn run_canvas_node(
    state: &AppState,
    user_id: i64,
    input: RunCanvasNodeInput,
) -> Result<CanvasRunSummary, String> {
    let prepared = prepare_canvas_single_node_run(state, user_id, input).await?;
    submit_prepared_canvas_graph_run(state, user_id, prepared)
}

pub async fn resume_canvas_graph_run(
    state: &AppState,
    user_id: i64,
    workflow_run_id: i64,
) -> Result<CanvasRunSummary, String> {
    let prepared = prepare_canvas_graph_resume(state, user_id, workflow_run_id).await?;
    if prepared.runnable_node_ids.is_empty() {
        return Ok(CanvasRunSummary {
            workflow_run_id: prepared.workflow_run_id,
            project_id: prepared.project_id,
            graph_id: prepared.graph_id,
            mode: prepared.mode,
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            error_message: None,
        });
    }
    submit_prepared_canvas_graph_run(state, user_id, prepared)
}

fn submit_prepared_canvas_graph_run(
    state: &AppState,
    user_id: i64,
    prepared: PreparedCanvasGraphRun,
) -> Result<CanvasRunSummary, String> {
    let summary = CanvasRunSummary {
        workflow_run_id: prepared.workflow_run_id,
        project_id: prepared.project_id,
        graph_id: prepared.graph_id,
        mode: prepared.mode.clone(),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        error_message: None,
    };
    let runtime_snapshot = crate::runtime_config::current_snapshot();
    let background_state = state.clone();
    tokio::spawn(async move {
        let workflow_run_id = prepared.workflow_run_id;
        let result = execute_prepared_canvas_graph_run_with_runtime_config(
            &background_state,
            user_id,
            prepared,
            runtime_snapshot,
        )
        .await;
        if let Err(error) = result {
            if canvas_workflow_run_is_cancelled(&background_state, workflow_run_id)
                .await
                .unwrap_or(false)
            {
                tracing::info!(
                    workflow_run_id,
                    error = %error,
                    "画布后台运行在取消后停止"
                );
                return;
            }
            if let Err(mark_error) = background_state
                .orchestrator
                .mark_workflow_status(workflow_run_id, WORKFLOW_STATUS_FAILED, true, Some(&error))
                .await
            {
                tracing::error!(
                    workflow_run_id,
                    error = %error,
                    mark_error = %mark_error,
                    "画布后台运行失败，且标记失败状态也失败"
                );
            } else {
                tracing::error!(
                    workflow_run_id,
                    error = %error,
                    "画布后台运行失败"
                );
            }
        }
    });
    Ok(summary)
}
