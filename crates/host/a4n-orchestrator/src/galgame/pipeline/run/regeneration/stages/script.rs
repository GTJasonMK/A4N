use a4n_error::AppError;
use serde_json::Value;

use crate::galgame::{
    GalgameBranchDesign, GalgameSceneRegenerationInput, GalgameScriptAdaptation,
    OrchestratorContext, galgame_script_adaptation_user_message, galgame_source_markers,
    script_adaptation_from_value,
};
use a4n_prompt::rendering::PromptRenderer;

use super::super::super::validation::ensure_single_regenerated_scene_lines;
use super::super::contracts::PreparedGalgameSceneRegeneration;
use crate::workflow::WorkflowLlmOnceInput;

pub(super) async fn run_script_adaptation_stage(
    service: &OrchestratorContext,
    input: &GalgameSceneRegenerationInput,
    prepared: &PreparedGalgameSceneRegeneration,
    branch: &(Value, GalgameBranchDesign),
    prompt_renderer: &dyn PromptRenderer,
) -> Result<(Value, GalgameScriptAdaptation), AppError> {
    let source_markers = galgame_source_markers(std::slice::from_ref(&prepared.prompt_chapter))?;
    service
        .run_workflow_llm_once(
            WorkflowLlmOnceInput {
                stage_key: "script_adaptation",
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
                    &branch.0,
                    &source_markers,
                )?,
                cancel_token: None,
            },
            |value| {
                let value = source_markers.decode_output(&value)?;
                let parsed = script_adaptation_from_value(value, "galgame_script_adaptation")?;
                ensure_single_regenerated_scene_lines(&parsed)?;
                crate::galgame::pipeline::persist::validate_galgame_script_adaptation(
                    &parsed,
                    &prepared.adaptation_projection,
                    &branch.1,
                    &prepared.persist_source_spans,
                )?;
                Ok(parsed)
            },
        )
        .await
}
