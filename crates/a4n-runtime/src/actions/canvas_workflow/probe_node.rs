use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::{
    CanvasGraphRecordDocument, CanvasNodeRunHandle, CanvasProbeNodeExecutionInput,
};

use super::CanvasRunSummary;

pub(in crate::actions::canvas_workflow) async fn run_canvas_probe_node(
    canvas: &a4n_orchestrator::OrchestratorContext,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
    run_handle: Option<CanvasNodeRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    canvas
        .run_canvas_probe_node_validated(CanvasProbeNodeExecutionInput {
            project_id,
            graph_id,
            node,
            document,
            shared_run: run_handle,
            mode,
        })
        .await
        .map_err(|error| error.to_string())
}
