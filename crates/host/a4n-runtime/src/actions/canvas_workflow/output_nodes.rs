use a4n_canvas_contract::{CanvasNodeRun, CanvasOutputNodeApplicationInput};

use crate::state::AppState;

use super::CanvasRunSummary;

pub(in crate::actions::canvas_workflow) async fn run_canvas_output_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node_id: i64,
    run_handle: Option<CanvasNodeRun>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    state
        .canvas_output_application()
        .run_output(CanvasOutputNodeApplicationInput {
            user_id,
            project_id,
            graph_id,
            node_id,
            shared_run: run_handle,
            mode: mode.to_string(),
        })
        .await
        .map_err(|error| error.to_string())
}
