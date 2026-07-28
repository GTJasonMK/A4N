use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::{
    CanvasTextNodeExecutionInput, CanvasTextNodeRunHandle, CanvasTextNodeRunSummary,
};

use crate::AppState;

use super::super::node_lifecycle::CanvasRunHandle;
use super::super::{CanvasGraphDocument, CanvasRunSummary};
use super::host_port::RuntimeCanvasTextGenerationLlmPort;

pub(in crate::actions::canvas_workflow) async fn run_canvas_render_text_node(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    state
        .canvas()
        .run_canvas_render_text_node_validated(CanvasTextNodeExecutionInput {
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

pub(in crate::actions::canvas_workflow) async fn run_canvas_llm_text_node(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let llm_port = RuntimeCanvasTextGenerationLlmPort::new(state);
    state
        .canvas()
        .run_canvas_llm_text_node_validated(
            CanvasTextNodeExecutionInput {
                project_id,
                graph_id,
                node,
                document,
                shared_run: run_handle.map(orchestrator_run_handle),
                mode,
            },
            &llm_port,
        )
        .await
        .map(runtime_summary)
        .map_err(|error| error.to_string())
}

fn orchestrator_run_handle(run: CanvasRunHandle) -> CanvasTextNodeRunHandle {
    CanvasTextNodeRunHandle {
        workflow_run_id: run.workflow_run_id,
        finalize_on_success: run.finalize_on_success,
    }
}

fn runtime_summary(summary: CanvasTextNodeRunSummary) -> CanvasRunSummary {
    CanvasRunSummary {
        workflow_run_id: summary.workflow_run_id,
        project_id: summary.project_id,
        graph_id: summary.graph_id,
        mode: summary.mode,
        status: summary.status,
        error_message: summary.error_message,
    }
}
