use a4n_canvas_contract::{
    CanvasGraphResumePreparationInput, CanvasGraphRunPreparationInput,
    CanvasSingleNodeRunPreparationInput, PreparedCanvasGraphRun,
};
use a4n_domain::agent::AgentWorkflowOrigin;
use a4n_orchestrator::workflow::{
    WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};

use super::host_port::RuntimeCanvasGraphExecutionHostPort;
use crate::{
    AppState,
    actions::canvas_workflow::{CanvasRunSummary, RunCanvasGraphInput, RunCanvasNodeInput},
};

pub async fn run_canvas_graph(
    state: &AppState,
    user_id: i64,
    input: RunCanvasGraphInput,
) -> Result<CanvasRunSummary, String> {
    run_canvas_graph_with_origin(state, user_id, input, None).await
}

pub(crate) async fn run_canvas_graph_with_origin(
    state: &AppState,
    user_id: i64,
    input: RunCanvasGraphInput,
    origin: Option<&AgentWorkflowOrigin>,
) -> Result<CanvasRunSummary, String> {
    let host_port = RuntimeCanvasGraphExecutionHostPort::new(state);
    let prepared = state
        .canvas_graph_execution_application()
        .prepare_graph(
            CanvasGraphRunPreparationInput {
                user_id,
                project_id: input.project_id,
                graph_id: input.graph_id,
                target_node_id: input.target_node_id,
            },
            &host_port,
        )
        .await?;
    submit_prepared_canvas_graph_run(state, prepared, origin).await
}

pub async fn run_canvas_node(
    state: &AppState,
    user_id: i64,
    input: RunCanvasNodeInput,
) -> Result<CanvasRunSummary, String> {
    run_canvas_node_with_origin(state, user_id, input, None).await
}

pub(crate) async fn run_canvas_node_with_origin(
    state: &AppState,
    user_id: i64,
    input: RunCanvasNodeInput,
    origin: Option<&AgentWorkflowOrigin>,
) -> Result<CanvasRunSummary, String> {
    let host_port = RuntimeCanvasGraphExecutionHostPort::new(state);
    let prepared = state
        .canvas_graph_execution_application()
        .prepare_single_node(
            CanvasSingleNodeRunPreparationInput {
                user_id,
                project_id: input.project_id,
                graph_id: input.graph_id,
                node_id: input.node_id,
            },
            &host_port,
        )
        .await?;
    submit_prepared_canvas_graph_run(state, prepared, origin).await
}

pub(crate) async fn resume_canvas_graph_run(
    state: &AppState,
    user_id: i64,
    workflow_run_id: i64,
) -> Result<CanvasRunSummary, String> {
    let host_port = RuntimeCanvasGraphExecutionHostPort::new(state);
    let prepared = state
        .canvas_graph_execution_application()
        .prepare_resume(
            CanvasGraphResumePreparationInput {
                user_id,
                workflow_run_id,
            },
            &host_port,
        )
        .await?;
    if prepared.runnable_node_ids.is_empty() {
        return Ok(prepared.summary(WORKFLOW_STATUS_COMPLETED, None));
    }
    submit_prepared_canvas_graph_run(state, prepared, None).await
}

async fn submit_prepared_canvas_graph_run(
    state: &AppState,
    prepared: PreparedCanvasGraphRun,
    origin: Option<&AgentWorkflowOrigin>,
) -> Result<CanvasRunSummary, String> {
    if let Some(origin) = origin
        && let Err(error) = state
            .agent_workflow_binding_application()
            .bind_owned_workflow(origin, prepared.workflow_run_id)
            .await
    {
        state
            .canvas_graph_execution_application()
            .fail_unsubmitted(prepared.workflow_run_id, &error.to_string())
            .await
            .map_err(|failure| format!("{error}；停止未派发的 Workflow 失败: {failure}"))?;
        return Err(error.to_string());
    }
    let summary = prepared.summary(WORKFLOW_STATUS_RUNNING, None);
    let background_state = state.clone();
    tokio::spawn(crate::runtime_config::inherit_runtime_context(async move {
        match run_prepared_canvas_graph(&background_state, prepared).await {
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
    }));
    Ok(summary)
}

async fn run_prepared_canvas_graph(
    state: &AppState,
    prepared: PreparedCanvasGraphRun,
) -> Result<CanvasRunSummary, String> {
    let host_port = RuntimeCanvasGraphExecutionHostPort::new(state);
    state
        .canvas_graph_execution_application()
        .run_prepared(prepared, &host_port)
        .await
}
