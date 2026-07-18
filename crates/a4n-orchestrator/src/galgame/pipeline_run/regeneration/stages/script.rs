use a4n_core::error::AppError;
use serde_json::Value;

use crate::galgame::{
    GalgameSceneRegenerationInput, OrchestratorContext, galgame_script_adaptation_user_message,
    script_adaptation_from_value,
};
use a4n_prompt::rendering::PromptRenderer;

use super::super::super::llm::GalgameLlmOnceInput;
use super::super::super::validation::ensure_single_regenerated_scene_lines;
use super::super::contracts::PreparedGalgameSceneRegeneration;

pub(super) async fn run_script_adaptation_stage(
    service: &OrchestratorContext,
    input: &GalgameSceneRegenerationInput,
    prepared: &PreparedGalgameSceneRegeneration,
    branch_plan_json: &Value,
    prompt_renderer: &dyn PromptRenderer,
) -> Result<Value, AppError> {
    let lines_draft_json = service
        .run_galgame_llm_once(
            GalgameLlmOnceInput {
                prompt_renderer,
                project_id: input.project_id,
                root_project_id: input.root_project_id,
                workflow_run_id: input.workflow_run_id,
                workflow: "source_sync_regenerate_scene",
                step_key: format!("source_sync.scene.{}.script_adaptation", input.scene_id),
                call_order: 1,
                prompt_name: "galgame_script_adaptation",
                user_prompt: galgame_script_adaptation_user_message(
                    std::slice::from_ref(&prepared.prompt_chapter),
                    &prepared.scenes_json,
                    branch_plan_json,
                )?,
            },
            |value| {
                let parsed = script_adaptation_from_value(value, "galgame_script_adaptation")?;
                serde_json::to_value(parsed).map_err(AppError::Serialization)
            },
        )
        .await?;
    let script_adaptation = script_adaptation_from_value(lines_draft_json.clone(), "lines_draft")?;
    ensure_single_regenerated_scene_lines(&script_adaptation)?;
    Ok(lines_draft_json)
}
