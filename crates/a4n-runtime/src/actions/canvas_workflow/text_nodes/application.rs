use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::{
    CanvasGraphRecordDocument, CanvasNodeRunHandle, CanvasTextNodeExecutionInput,
};

use crate::AppState;

use super::super::CanvasRunSummary;
use super::host_port::RuntimeCanvasTextGenerationLlmPort;

pub(in crate::actions::canvas_workflow) async fn run_canvas_render_text_node(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
    run_handle: Option<CanvasNodeRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    state
        .orchestrator()
        .run_canvas_render_text_node_validated(CanvasTextNodeExecutionInput {
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

pub(in crate::actions::canvas_workflow) async fn run_canvas_llm_text_node(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
    run_handle: Option<CanvasNodeRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let llm_port = RuntimeCanvasTextGenerationLlmPort::new(state);
    state
        .orchestrator()
        .run_canvas_llm_text_node_validated(
            CanvasTextNodeExecutionInput {
                project_id,
                graph_id,
                node,
                document,
                shared_run: run_handle,
                mode,
            },
            &llm_port,
        )
        .await
        .map_err(|error| error.to_string())
}
