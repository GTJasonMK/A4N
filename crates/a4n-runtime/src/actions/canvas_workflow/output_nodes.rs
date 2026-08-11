use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::CanvasOutputNodeExecutionInput;

use super::{
    CanvasGraphDocument, CanvasRunSummary,
    node_lifecycle::{CanvasRunHandle, runtime_summary},
};

pub(in crate::actions::canvas_workflow) async fn run_canvas_output_node(
    canvas: &a4n_orchestrator::OrchestratorContext,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    canvas
        .run_canvas_output_node_validated(CanvasOutputNodeExecutionInput {
            project_id,
            graph_id,
            node,
            document,
            shared_run: run_handle.map(Into::into),
            mode,
        })
        .await
        .map(runtime_summary)
        .map_err(|error| error.to_string())
}
