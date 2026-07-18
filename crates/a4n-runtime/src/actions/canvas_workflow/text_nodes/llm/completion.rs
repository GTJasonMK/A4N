use super::super::super::CanvasRunSummary;
use super::super::super::execution::record_canvas_node_checkpoint;
use super::super::super::node_lifecycle::finish_canvas_node_success;
use super::super::super::persistence::{
    mark_canvas_generation_spec_status, upsert_canvas_prompt_artifact,
};
use super::contracts::{CanvasLlmNodeLifecycle, ValidatedCanvasLlmText};
use crate::AppState;
use a4n_orchestrator::workflow::WORKFLOW_STATUS_COMPLETED;
use serde_json::json;

pub(super) async fn complete_canvas_llm_text_node(
    state: &AppState,
    lifecycle: CanvasLlmNodeLifecycle,
    output_kind: &str,
    validated: ValidatedCanvasLlmText,
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
    mark_canvas_generation_spec_status(state, generation_spec_id, WORKFLOW_STATUS_COMPLETED)
        .await?;
    let artifact = upsert_canvas_prompt_artifact(
        state,
        generation_spec_id,
        output_kind,
        &validated.transformed_text,
    )
    .await?;
    record_canvas_node_checkpoint(
        state,
        run.workflow_run_id,
        step_key,
        WORKFLOW_STATUS_COMPLETED,
        input_json,
        Some(json!({
            "data_type": "text",
            "text": artifact.prompt_text,
            "generation_spec_id": generation_spec_id,
            "prompt_artifact_id": artifact.id,
            "prompt_kind": artifact.prompt_kind,
            "llm_finish_reason": validated.finish_reason,
            "llm_usage": validated.usage})),
        None,
        true,
    )
    .await?;
    finish_canvas_node_success(state, run, project_id, graph_id, &mode).await
}
