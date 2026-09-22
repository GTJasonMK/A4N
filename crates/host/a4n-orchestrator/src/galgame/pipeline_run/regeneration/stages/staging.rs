use a4n_error::AppError;
use serde_json::Value;

use crate::galgame::{
    GalgameCharacterStaging, GalgameSceneRegenerationInput, GalgameScriptAdaptation,
    OrchestratorContext, character_staging_from_value, galgame_character_staging_user_message,
};
use a4n_prompt::rendering::PromptRenderer;

use super::super::super::llm::GalgameLlmOnceInput;
use super::super::super::validation::ensure_single_regenerated_staged_lines;

pub(super) async fn run_character_staging_stage(
    service: &OrchestratorContext,
    input: &GalgameSceneRegenerationInput,
    script: &(Value, GalgameScriptAdaptation),
    prompt_renderer: &dyn PromptRenderer,
) -> Result<(Value, GalgameCharacterStaging), AppError> {
    service
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
                    &script.0,
                    &input.speaker_profiles,
                )?,
            },
            |value| {
                let parsed = character_staging_from_value(value, "galgame_character_staging")?;
                ensure_single_regenerated_staged_lines(&parsed)?;
                crate::galgame::pipeline_persist::validate_galgame_staging(
                    &parsed,
                    &script.1,
                    &input.speaker_profiles,
                )?;
                Ok(parsed)
            },
        )
        .await
}
