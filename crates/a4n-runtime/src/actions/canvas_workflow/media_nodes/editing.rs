mod host_port;

use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::{
    CanvasEditingNodeExecutionInput, CanvasNodeRunHandle, CanvasNodeRunSummary,
};

use super::super::{CanvasGraphDocument, CanvasRunSummary, node_lifecycle::CanvasRunHandle};
use crate::state::AppState;

use host_port::RuntimeCanvasEditingNodeHostPort;

pub(in crate::actions::canvas_workflow) async fn run_canvas_extract_frame_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    run_canvas_editing_node(
        state, user_id, project_id, graph_id, node, document, run_handle, mode,
    )
    .await
}

pub(in crate::actions::canvas_workflow) async fn run_canvas_trim_video_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    run_canvas_editing_node(
        state, user_id, project_id, graph_id, node, document, run_handle, mode,
    )
    .await
}

pub(in crate::actions::canvas_workflow) async fn run_canvas_assemble_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    run_canvas_editing_node(
        state, user_id, project_id, graph_id, node, document, run_handle, mode,
    )
    .await
}

async fn run_canvas_editing_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let shared_run = run_handle.map(|run| CanvasNodeRunHandle {
        workflow_run_id: run.workflow_run_id,
        finalize_on_success: run.finalize_on_success,
    });
    let host_port = RuntimeCanvasEditingNodeHostPort::new(state);
    let summary = state
        .canvas()
        .run_canvas_editing_node_validated(
            CanvasEditingNodeExecutionInput {
                user_id,
                project_id,
                graph_id,
                node,
                document,
                shared_run,
                mode,
            },
            &host_port,
        )
        .await
        .map_err(|error| error.to_string())?;
    Ok(canvas_run_summary(summary))
}

fn canvas_run_summary(summary: CanvasNodeRunSummary) -> CanvasRunSummary {
    CanvasRunSummary {
        workflow_run_id: summary.workflow_run_id,
        project_id: summary.project_id,
        graph_id: summary.graph_id,
        mode: summary.mode,
        status: summary.status,
        error_message: summary.error_message,
    }
}
