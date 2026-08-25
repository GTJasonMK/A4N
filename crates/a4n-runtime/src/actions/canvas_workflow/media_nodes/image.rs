use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::{
    canvas::{CanvasGraphRecordDocument, CanvasImageNodeExecutionInput, CanvasNodeRunHandle},
    media::ImageGenerationProgress,
};

use crate::{AppState, actions::image_generation::RuntimeImageGenerationHostPort};

use super::super::CanvasRunSummary;

pub(in crate::actions::canvas_workflow) async fn run_canvas_generate_image_node(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
    run_handle: Option<CanvasNodeRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let host_port = RuntimeImageGenerationHostPort::new(state);
    state
        .orchestrator()
        .run_canvas_image_node_validated(
            CanvasImageNodeExecutionInput {
                project_id,
                graph_id,
                node,
                document,
                shared_run: run_handle,
                mode,
            },
            &host_port,
            &ImageGenerationProgress::none(),
        )
        .await
        .map_err(|error| error.to_string())
}
