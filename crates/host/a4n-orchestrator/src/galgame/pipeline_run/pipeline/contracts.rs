use serde_json::Value;

use crate::galgame::{
    GalgameBranchDesign, GalgameCharacterStaging, GalgamePipelinePromptContext,
    GalgamePipelineRunOptions, GalgamePipelineStartStage, GalgameShootingScriptProjection,
    GalgameVisualPlan, OrchestratorContext, should_restore_galgame_stage_checkpoint,
};
use a4n_prompt::rendering::PromptRenderer;

pub(super) struct GalgamePipelineExecution<'a> {
    pub(super) service: &'a OrchestratorContext,
    pub(super) project_id: i64,
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) options: &'a GalgamePipelineRunOptions,
    pub(super) prompt_context: &'a GalgamePipelinePromptContext,
}

impl GalgamePipelineExecution<'_> {
    pub(super) fn should_restore(&self, stage: GalgamePipelineStartStage) -> bool {
        should_restore_galgame_stage_checkpoint(
            self.options.resume_from_checkpoint,
            self.options.start_from_stage,
            stage.order(),
        )
    }

    pub(super) fn restore_required(&self, stage: GalgamePipelineStartStage) -> bool {
        self.options
            .start_from_stage
            .is_some_and(|start| stage.order() < start.order())
    }
}

pub(super) struct GalgamePipelineEarlyOutputs {
    pub(super) scenes_json: Value,
    pub(super) scenes: GalgameShootingScriptProjection,
    pub(super) branch_plan: GalgameBranchDesign,
    pub(super) lines_staged_json: Value,
    pub(super) staging: GalgameCharacterStaging,
}

pub(super) struct GalgamePipelineLateOutputs {
    pub(super) visual_plan: GalgameVisualPlan,
}
