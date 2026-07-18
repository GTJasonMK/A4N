use crate::AppState;
use crate::actions::app_error_message;
use a4n_core::models::generation::prompt_artifact;
use a4n_orchestrator::canvas::CanvasPromptArtifactUpsertInput;

pub(in crate::actions::canvas_workflow) async fn upsert_canvas_prompt_artifact(
    state: &AppState,
    generation_spec_id: i64,
    prompt_kind: &str,
    prompt_text: &str,
) -> Result<prompt_artifact::Model, String> {
    state
        .orchestrator
        .upsert_canvas_prompt_artifact_record(CanvasPromptArtifactUpsertInput {
            generation_spec_id,
            prompt_kind: prompt_kind.to_string(),
            prompt_text: prompt_text.to_string(),
        })
        .await
        .map_err(app_error_message)
}
