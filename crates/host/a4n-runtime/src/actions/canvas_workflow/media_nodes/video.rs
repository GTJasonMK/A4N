use a4n_canvas_contract::{CanvasNodeRun, CanvasVideoNodeApplicationInput};

use crate::{AppState, actions::video_generation::RuntimeVideoProviderHostPort};

use super::super::CanvasRunSummary;

pub(in crate::actions::canvas_workflow) async fn run_canvas_generate_video_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node_id: i64,
    run_handle: Option<CanvasNodeRun>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let host_port = RuntimeVideoProviderHostPort::new(state);
    state
        .canvas_video_application()
        .run_video(
            CanvasVideoNodeApplicationInput {
                user_id,
                project_id,
                graph_id,
                node_id,
                shared_run: run_handle,
                mode: mode.to_string(),
            },
            &host_port,
        )
        .await
        .map_err(|error| error.to_string())
}
