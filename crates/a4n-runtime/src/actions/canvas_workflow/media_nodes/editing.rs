mod host_port;

use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::CanvasEditingNodeExecutionInput;

use super::super::{
    CanvasGraphDocument, CanvasRunSummary,
    node_lifecycle::{CanvasRunHandle, runtime_summary},
};
use crate::state::AppState;

use host_port::RuntimeCanvasEditingNodeHostPort;

pub(in crate::actions::canvas_workflow) async fn run_canvas_editing_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let shared_run = run_handle.map(Into::into);
    let host_port = RuntimeCanvasEditingNodeHostPort::new(state);
    let summary = state
        .orchestrator()
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
    Ok(runtime_summary(summary))
}
