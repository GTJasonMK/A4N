use super::request::galgame_preview_step;
use crate::AppState;
use crate::actions::request_preview::ModelRequestPreviewStep;
use a4n_orchestrator::galgame::{
    GalgamePipelinePromptContext, GalgamePipelineStartStage, galgame_branch_design_user_message,
    galgame_character_staging_user_message, galgame_script_adaptation_user_message,
    galgame_visual_planning_user_message,
};
use serde_json::{Value, json};

pub(super) struct GalgamePreviewStepInputs<'a> {
    start_from_stage: Option<GalgamePipelineStartStage>,
    context: &'a GalgamePipelinePromptContext,
    shooting_script_projection: &'a Value,
    branch_design: Option<&'a Value>,
    script_adaptation: Option<&'a Value>,
    character_staging: Option<&'a Value>,
}

impl<'a> GalgamePreviewStepInputs<'a> {
    pub(super) fn new(
        start_from_stage: Option<GalgamePipelineStartStage>,
        context: &'a GalgamePipelinePromptContext,
        shooting_script_projection: &'a Value,
        branch_design: Option<&'a Value>,
        script_adaptation: Option<&'a Value>,
        character_staging: Option<&'a Value>,
    ) -> Self {
        Self {
            start_from_stage,
            context,
            shooting_script_projection,
            branch_design,
            script_adaptation,
            character_staging,
        }
    }
}

pub(super) async fn build_galgame_preview_steps(
    state: &AppState,
    inputs: GalgamePreviewStepInputs<'_>,
) -> Result<Vec<ModelRequestPreviewStep>, String> {
    let mut steps = Vec::new();
    let stage_filter = |stage: GalgamePipelineStartStage| {
        inputs
            .start_from_stage
            .map(|start| stage.order() >= start.order())
            .unwrap_or(true)
    };

    if stage_filter(GalgamePipelineStartStage::BranchDesign) {
        steps.push(
            galgame_preview_step(
                state,
                "branch_design",
                "分支设计",
                "galgame_branch_design",
                galgame_branch_design_user_message(inputs.shooting_script_projection)
                    .map_err(|err| err.to_string())?,
            )
            .await?,
        );
    }
    if stage_filter(GalgamePipelineStartStage::ScriptAdaptation) {
        let branch_design_input = inputs
            .branch_design
            .cloned()
            .unwrap_or_else(|| json!({"choices": []}));
        steps.push(
            galgame_preview_step(
                state,
                "script_adaptation",
                "剧本行改编",
                "galgame_script_adaptation",
                galgame_script_adaptation_user_message(
                    &inputs.context.chapters,
                    inputs.shooting_script_projection,
                    &branch_design_input,
                )
                .map_err(|err| err.to_string())?,
            )
            .await?,
        );
    }
    if stage_filter(GalgamePipelineStartStage::CharacterStaging)
        && let Some(script_adaptation_input) = inputs.script_adaptation
    {
        steps.push(
            galgame_preview_step(
                state,
                "character_staging",
                "角色演出标注",
                "galgame_character_staging",
                galgame_character_staging_user_message(
                    script_adaptation_input,
                    &inputs.context.speaker_profiles,
                )
                .map_err(|err| err.to_string())?,
            )
            .await?,
        );
    }
    if stage_filter(GalgamePipelineStartStage::VisualPlanning)
        && let Some(character_staging_input) = inputs.character_staging
    {
        steps.push(
            galgame_preview_step(
                state,
                "visual_planning",
                "视觉资产规划",
                "galgame_visual_planning",
                galgame_visual_planning_user_message(
                    inputs.shooting_script_projection,
                    character_staging_input,
                )
                .map_err(|err| err.to_string())?,
            )
            .await?,
        );
    }
    Ok(steps)
}
