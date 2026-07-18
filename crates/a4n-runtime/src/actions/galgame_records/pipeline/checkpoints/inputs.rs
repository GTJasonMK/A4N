use a4n_orchestrator::galgame::GalgamePipelineStartStage;
use serde_json::Value;

#[derive(Default)]
pub(in crate::actions::galgame_records::pipeline) struct GalgamePreviewCheckpointInputs {
    pub(in crate::actions::galgame_records::pipeline) scenes: Option<Value>,
    pub(in crate::actions::galgame_records::pipeline) lines_draft: Option<Value>,
    pub(in crate::actions::galgame_records::pipeline) lines_staged: Option<Value>,
    pub(in crate::actions::galgame_records::pipeline) visual_plan: Option<Value>,
    pub(in crate::actions::galgame_records::pipeline) branch_plan: Option<Value>,
    pub(in crate::actions::galgame_records::pipeline) required_prior_stage:
        Option<GalgamePipelineStartStage>,
}

impl GalgamePreviewCheckpointInputs {
    pub(in crate::actions::galgame_records::pipeline) fn shooting_script_projection_input(
        &self,
    ) -> Result<Option<Value>, String> {
        self.prior_stage_input(
            GalgamePipelineStartStage::ShootingScriptProjection,
            self.scenes.clone(),
        )
    }

    pub(in crate::actions::galgame_records::pipeline) fn script_adaptation_input(
        &self,
    ) -> Result<Option<Value>, String> {
        self.prior_stage_input(
            GalgamePipelineStartStage::ScriptAdaptation,
            self.lines_draft.clone(),
        )
    }

    pub(in crate::actions::galgame_records::pipeline) fn branch_design_input(
        &self,
    ) -> Result<Option<Value>, String> {
        self.prior_stage_input(
            GalgamePipelineStartStage::BranchDesign,
            self.branch_plan.clone(),
        )
    }

    pub(in crate::actions::galgame_records::pipeline) fn character_staging_input(
        &self,
    ) -> Result<Option<Value>, String> {
        self.prior_stage_input(
            GalgamePipelineStartStage::CharacterStaging,
            self.lines_staged.clone(),
        )
    }

    fn prior_stage_input(
        &self,
        stage: GalgamePipelineStartStage,
        value: Option<Value>,
    ) -> Result<Option<Value>, String> {
        if self
            .required_prior_stage
            .is_some_and(|start| stage.order() < start.order())
        {
            return value.map(Some).ok_or_else(|| {
                format!(
                    "Galgame 请求预览缺少上游 checkpoint: start_from_stage={}, required_stage={}",
                    start_stage_key(self.required_prior_stage),
                    stage.key()
                )
            });
        }
        Ok(value)
    }

    pub(in crate::actions::galgame_records::pipeline) fn validate_required_prior_checkpoints(
        &self,
    ) -> Result<(), String> {
        let Some(start) = self.required_prior_stage else {
            return Ok(());
        };
        let required = [
            (
                GalgamePipelineStartStage::ShootingScriptProjection,
                &self.scenes,
            ),
            (GalgamePipelineStartStage::BranchDesign, &self.branch_plan),
            (
                GalgamePipelineStartStage::ScriptAdaptation,
                &self.lines_draft,
            ),
            (
                GalgamePipelineStartStage::CharacterStaging,
                &self.lines_staged,
            ),
            (GalgamePipelineStartStage::VisualPlanning, &self.visual_plan),
        ];
        for (stage, value) in required {
            if stage.order() < start.order() && value.is_none() {
                return Err(format!(
                    "Galgame 请求预览缺少上游 checkpoint: start_from_stage={}, required_stage={}",
                    start.key(),
                    stage.key()
                ));
            }
        }
        Ok(())
    }
}

fn start_stage_key(stage: Option<GalgamePipelineStartStage>) -> &'static str {
    stage
        .map(|stage| stage.key())
        .unwrap_or("shooting_script_projection")
}
