use a4n_canvas_contract::{CanvasLlmTextNodeApplicationInput, CanvasNodeRun};

use crate::AppState;

use super::super::CanvasRunSummary;
use super::host_port::RuntimeCanvasTextGenerationLlmPort;

pub(in crate::actions::canvas_workflow) async fn run_canvas_llm_text_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node_id: i64,
    run_handle: Option<CanvasNodeRun>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let llm_port = RuntimeCanvasTextGenerationLlmPort::new(state);
    state
        .canvas_llm_text_application()
        .run_llm_text(
            CanvasLlmTextNodeApplicationInput {
                user_id,
                project_id,
                graph_id,
                node_id,
                shared_run: run_handle,
                mode: mode.to_string(),
            },
            &llm_port,
        )
        .await
        .map_err(|error| error.to_string())
}
