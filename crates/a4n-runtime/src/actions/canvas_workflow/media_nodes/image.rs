use a4n_canvas_contract::{CanvasImageNodeApplicationInput, CanvasNodeRun};
use a4n_orchestrator::media::ImageGenerationProgress;

use crate::{AppState, actions::image_generation::RuntimeImageGenerationHostPort};

use super::super::CanvasRunSummary;

pub(in crate::actions::canvas_workflow) async fn run_canvas_generate_image_node(
    state: &AppState,
    requested_by_user_id: i64,
    project_id: i64,
    graph_id: i64,
    node_id: i64,
    run_handle: Option<CanvasNodeRun>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let host_port = RuntimeImageGenerationHostPort::new(state);
    state
        .canvas_image_application()
        .run_image(
            CanvasImageNodeApplicationInput {
                user_id: requested_by_user_id,
                project_id,
                graph_id,
                node_id,
                shared_run: run_handle,
                mode: mode.to_string(),
            },
            &host_port,
            &ImageGenerationProgress::none(),
        )
        .await
        .map_err(|error| error.to_string())
}
