use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::{
    CanvasNodeRunHandle, CanvasNodeRunSummary, CanvasProbeNodeExecutionInput,
};

use crate::state::CanvasRuntimeFacade;

use super::{CanvasGraphDocument, CanvasRunSummary, node_lifecycle::CanvasRunHandle};

pub(in crate::actions::canvas_workflow) async fn run_canvas_probe_node(
    canvas: &CanvasRuntimeFacade<'_>,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    canvas
        .run_canvas_probe_node_validated(CanvasProbeNodeExecutionInput {
            project_id,
            graph_id,
            node,
            document,
            shared_run: run_handle.map(orchestrator_run_handle),
            mode,
        })
        .await
        .map(runtime_summary)
        .map_err(|error| error.to_string())
}

fn orchestrator_run_handle(run: CanvasRunHandle) -> CanvasNodeRunHandle {
    CanvasNodeRunHandle {
        workflow_run_id: run.workflow_run_id,
        finalize_on_success: run.finalize_on_success,
    }
}

fn runtime_summary(summary: CanvasNodeRunSummary) -> CanvasRunSummary {
    CanvasRunSummary {
        workflow_run_id: summary.workflow_run_id,
        project_id: summary.project_id,
        graph_id: summary.graph_id,
        mode: summary.mode,
        status: summary.status,
        error_message: summary.error_message,
    }
}
