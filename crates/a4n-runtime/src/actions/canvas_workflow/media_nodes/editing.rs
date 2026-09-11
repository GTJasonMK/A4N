mod host_port;

use a4n_canvas_contract::{CanvasEditingNodeApplicationInput, CanvasEditingNodeRun};

use super::super::CanvasRunSummary;
use crate::state::AppState;

use host_port::RuntimeCanvasEditingNodeHostPort;

pub(in crate::actions::canvas_workflow) async fn run_canvas_editing_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node_id: i64,
    shared_run: Option<CanvasEditingNodeRun>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let host_port = RuntimeCanvasEditingNodeHostPort::new(state);
    let summary = state
        .canvas_editing_application()
        .run_editing(
            CanvasEditingNodeApplicationInput {
                user_id,
                project_id,
                graph_id,
                shared_run,
                node_id,
                mode: mode.to_string(),
            },
            &host_port,
        )
        .await
        .map_err(|error| error.to_string())?;
    Ok(summary)
}
