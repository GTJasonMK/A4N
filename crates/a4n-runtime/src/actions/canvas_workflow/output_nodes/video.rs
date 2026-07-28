use a4n_core::models::canvas::canvas_node;

use crate::state::CanvasRuntimeFacade;

use super::super::{CanvasGraphDocument, CanvasRunSummary, node_lifecycle::CanvasRunHandle};

pub(in crate::actions::canvas_workflow) async fn run_canvas_export_video_node(
    canvas: &CanvasRuntimeFacade<'_>,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    super::run_canvas_output_node_application(
        canvas, project_id, graph_id, node, document, run_handle, mode,
    )
    .await
}
