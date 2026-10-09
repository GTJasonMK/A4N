use a4n_error::AppError;
use serde_json::Value;

use crate::galgame::{
    GalgameBranchDesign, GalgameCharacterStaging, GalgameSceneRegenerationInput, GalgameVisualPlan,
    OrchestratorContext, galgame_source_markers, galgame_visual_planning_user_message,
    visual_plan_from_value,
};
use a4n_prompt::rendering::PromptRenderer;

use super::super::super::validation::ensure_single_regenerated_visual_plan;
use super::super::contracts::PreparedGalgameSceneRegeneration;
use crate::workflow::WorkflowLlmOnceInput;

pub(super) async fn run_visual_planning_stage(
    service: &OrchestratorContext,
    input: &GalgameSceneRegenerationInput,
    prepared: &PreparedGalgameSceneRegeneration,
    staging: &(Value, GalgameCharacterStaging),
    branch: &GalgameBranchDesign,
    prompt_renderer: &dyn PromptRenderer,
) -> Result<(Value, GalgameVisualPlan), AppError> {
    let source_markers = galgame_source_markers(std::slice::from_ref(&prepared.prompt_chapter))?;
    service
        .run_workflow_llm_once(
            WorkflowLlmOnceInput {
                stage_key: "visual_planning",
                prompt_renderer,
                project_id: input.project_id,
                root_project_id: input.root_project_id,
                workflow_run_id: input.workflow_run_id,
                workflow: "source_sync_regenerate_scene",
                step_key: format!("source_sync.scene.{}.visual_planning", input.scene_id),
                call_order: 3,
                prompt_name: "galgame_visual_planning",
                prompt_assembly_call_key: "GalgameService.build_source_sync_scene_regeneration",
                user_prompt: galgame_visual_planning_user_message(
                    &prepared.scenes_json,
                    &staging.0,
                    &source_markers,
                )?,
                cancel_token: None,
            },
            |value| {
                let value = source_markers.decode_output(&value)?;
                let parsed = visual_plan_from_value(value, "galgame_visual_planning")?;
                ensure_single_regenerated_visual_plan(&parsed)?;
                crate::galgame::build_galgame_script_persist_plan(
                    &prepared.adaptation_projection,
                    &staging.1,
                    &parsed,
                    branch,
                    &prepared.persist_source_spans,
                )?;
                Ok(parsed)
            },
        )
        .await
}
