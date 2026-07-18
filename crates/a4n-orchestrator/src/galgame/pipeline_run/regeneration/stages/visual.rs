use a4n_core::error::AppError;
use serde_json::Value;

use crate::galgame::{
    GalgameSceneRegenerationInput, GalgameVisualPlan, OrchestratorContext,
    galgame_visual_planning_user_message, visual_plan_from_value,
};
use a4n_prompt::rendering::PromptRenderer;

use super::super::super::llm::GalgameLlmOnceInput;
use super::super::super::validation::normalize_single_scene_visual_plan;
use super::super::contracts::PreparedGalgameSceneRegeneration;

pub(super) async fn run_visual_planning_stage(
    service: &OrchestratorContext,
    input: &GalgameSceneRegenerationInput,
    prepared: &PreparedGalgameSceneRegeneration,
    lines_staged_json: &Value,
    prompt_renderer: &dyn PromptRenderer,
) -> Result<(Value, GalgameVisualPlan), AppError> {
    let visual_plan_json = service
        .run_galgame_llm_once(
            GalgameLlmOnceInput {
                prompt_renderer,
                project_id: input.project_id,
                root_project_id: input.root_project_id,
                workflow_run_id: input.workflow_run_id,
                workflow: "source_sync_regenerate_scene",
                step_key: format!("source_sync.scene.{}.visual_planning", input.scene_id),
                call_order: 3,
                prompt_name: "galgame_visual_planning",
                user_prompt: galgame_visual_planning_user_message(
                    &prepared.scenes_json,
                    lines_staged_json,
                )?,
            },
            |value| {
                let parsed = visual_plan_from_value(value, "galgame_visual_planning")?;
                serde_json::to_value(parsed).map_err(AppError::Serialization)
            },
        )
        .await?;
    let visual_plan = visual_plan_from_value(visual_plan_json.clone(), "visual_plan")?;
    let visual_plan = normalize_single_scene_visual_plan(visual_plan)?;
    Ok((visual_plan_json, visual_plan))
}
