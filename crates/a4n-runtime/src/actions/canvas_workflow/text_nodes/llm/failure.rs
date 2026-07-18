use super::super::super::CanvasRunSummary;
use super::super::super::execution::finish_canvas_run_failed;
use super::super::super::persistence::mark_canvas_generation_spec_status;
use super::contracts::CanvasLlmNodeLifecycle;
use crate::AppState;
use a4n_orchestrator::workflow::WORKFLOW_STATUS_FAILED;

pub(super) async fn fail_canvas_llm_text_node(
    state: &AppState,
    lifecycle: CanvasLlmNodeLifecycle,
    error: String,
) -> Result<CanvasRunSummary, String> {
    let CanvasLlmNodeLifecycle {
        run,
        project_id,
        graph_id,
        mode,
        step_key,
        input_json,
        generation_spec_id,
    } = lifecycle;
    mark_canvas_generation_spec_status(state, generation_spec_id, WORKFLOW_STATUS_FAILED).await?;
    finish_canvas_run_failed(
        state,
        run.workflow_run_id,
        project_id,
        graph_id,
        &mode,
        step_key,
        input_json,
        error,
    )
    .await
}
