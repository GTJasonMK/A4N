use a4n_core::error::AppError;
use serde_json::Value;

use crate::galgame::{
    GalgameCharacterStaging, GalgameSceneRegenerationInput, OrchestratorContext,
    character_staging_from_value, galgame_character_staging_user_message,
};
use a4n_prompt::rendering::PromptRenderer;

use super::super::super::llm::GalgameLlmOnceInput;
use super::super::super::validation::ensure_single_regenerated_staged_lines;

pub(super) async fn run_character_staging_stage(
    service: &OrchestratorContext,
    input: &GalgameSceneRegenerationInput,
    lines_draft_json: &Value,
    prompt_renderer: &dyn PromptRenderer,
) -> Result<(Value, GalgameCharacterStaging), AppError> {
    let lines_staged_json = service
        .run_galgame_llm_once(
            GalgameLlmOnceInput {
                prompt_renderer,
                project_id: input.project_id,
                root_project_id: input.root_project_id,
                workflow_run_id: input.workflow_run_id,
                workflow: "source_sync_regenerate_scene",
                step_key: format!("source_sync.scene.{}.character_staging", input.scene_id),
                call_order: 2,
                prompt_name: "galgame_character_staging",
                user_prompt: galgame_character_staging_user_message(
                    lines_draft_json,
                    &input.speaker_profiles,
                )?,
            },
            |value| {
                let parsed = character_staging_from_value(value, "galgame_character_staging")?;
                serde_json::to_value(parsed).map_err(AppError::Serialization)
            },
        )
        .await?;
    let staging = character_staging_from_value(lines_staged_json.clone(), "lines_staged")?;
    ensure_single_regenerated_staged_lines(&staging)?;
    Ok((lines_staged_json, staging))
}
