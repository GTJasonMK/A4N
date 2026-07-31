use a4n_orchestrator::{
    canvas::{
        CanvasGraphResumePreparationInput, CanvasGraphRunPreparationInput, CanvasNodeRunSummary,
        CanvasSingleNodeRunPreparationInput, PreparedCanvasGraphRun,
    },
    workflow::{
        WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
        WORKFLOW_STATUS_RUNNING,
    },
};

use super::{host_port::RuntimeCanvasGraphExecutionHostPort, prelude::*};
use crate::actions::canvas_workflow::node_lifecycle::runtime_summary;

pub async fn run_canvas_graph(
    state: &AppState,
    user_id: i64,
    input: RunCanvasGraphInput,
) -> Result<CanvasRunSummary, String> {
    let host_port = RuntimeCanvasGraphExecutionHostPort::new(state);
    let prepared = state
        .canvas()
        .prepare_canvas_graph_run_validated(
            CanvasGraphRunPreparationInput {
                user_id,
                project_id: input.project_id,
                graph_id: input.graph_id,
                target_node_id: input.target_node_id,
            },
            &host_port,
        )
        .await?;
    submit_prepared_canvas_graph_run(state, prepared)
}

pub async fn run_canvas_node(
    state: &AppState,
    user_id: i64,
    input: RunCanvasNodeInput,
) -> Result<CanvasRunSummary, String> {
    let host_port = RuntimeCanvasGraphExecutionHostPort::new(state);
    let prepared = state
        .canvas()
        .prepare_canvas_single_node_run_validated(
            CanvasSingleNodeRunPreparationInput {
                user_id,
                project_id: input.project_id,
                graph_id: input.graph_id,
                node_id: input.node_id,
            },
            &host_port,
        )
        .await?;
    submit_prepared_canvas_graph_run(state, prepared)
}

pub async fn resume_canvas_graph_run(
    state: &AppState,
    user_id: i64,
    workflow_run_id: i64,
) -> Result<CanvasRunSummary, String> {
    let host_port = RuntimeCanvasGraphExecutionHostPort::new(state);
    let prepared = state
        .canvas()
        .prepare_canvas_graph_resume_validated(
            CanvasGraphResumePreparationInput {
                user_id,
                workflow_run_id,
            },
            &host_port,
        )
        .await?;
    if prepared.runnable_node_ids.is_empty() {
        return Ok(runtime_summary(
            prepared.summary(WORKFLOW_STATUS_COMPLETED, None),
        ));
    }
    submit_prepared_canvas_graph_run(state, prepared)
}

fn submit_prepared_canvas_graph_run(
    state: &AppState,
    prepared: PreparedCanvasGraphRun,
) -> Result<CanvasRunSummary, String> {
    let summary = runtime_summary(prepared.summary(WORKFLOW_STATUS_RUNNING, None));
    let runtime_snapshot = crate::runtime_config::current_snapshot();
    let background_state = state.clone();
    tokio::spawn(async move {
        match run_prepared_canvas_graph_with_runtime_config(
            &background_state,
            prepared,
            runtime_snapshot,
        )
        .await
        {
            Ok(summary) if summary.status == WORKFLOW_STATUS_CANCELLED => {
                tracing::info!(
                    workflow_run_id = summary.workflow_run_id,
                    "画布后台运行在取消后停止"
                );
            }
            Ok(summary) if summary.status == WORKFLOW_STATUS_FAILED => {
                tracing::error!(
                    workflow_run_id = summary.workflow_run_id,
                    error = summary.error_message.as_deref().unwrap_or("画布运行失败"),
                    "画布后台运行失败"
                );
            }
            Ok(_) => {}
            Err(error) => {
                tracing::error!(error = %error, "画布后台运行失败，且终态写入未能完成");
            }
        }
    });
    Ok(summary)
}

async fn run_prepared_canvas_graph_with_runtime_config(
    state: &AppState,
    prepared: PreparedCanvasGraphRun,
    runtime_snapshot: Option<crate::runtime_config::RuntimeConfigSnapshot>,
) -> Result<CanvasNodeRunSummary, String> {
    match runtime_snapshot {
        Some(snapshot) => {
            crate::runtime_config::scope(snapshot, run_prepared_canvas_graph(state, prepared)).await
        }
        None => run_prepared_canvas_graph(state, prepared).await,
    }
}

async fn run_prepared_canvas_graph(
    state: &AppState,
    prepared: PreparedCanvasGraphRun,
) -> Result<CanvasNodeRunSummary, String> {
    let host_port = RuntimeCanvasGraphExecutionHostPort::new(state);
    state
        .canvas()
        .run_prepared_canvas_graph_validated(prepared, &host_port)
        .await
}
