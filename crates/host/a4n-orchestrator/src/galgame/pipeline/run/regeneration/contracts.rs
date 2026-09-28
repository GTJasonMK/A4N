use serde_json::Value;

use crate::galgame::{
    GalgameAdaptationProjection, GalgameBranchDesign, GalgameCharacterStaging,
    GalgamePipelinePromptChapter, GalgamePipelineSourceSpan, GalgameVisualPlan,
};

pub(super) struct PreparedGalgameSceneRegeneration {
    pub(super) prompt_chapter: GalgamePipelinePromptChapter,
    pub(super) persist_source_spans: Vec<GalgamePipelineSourceSpan>,
    pub(super) adaptation_projection: GalgameAdaptationProjection,
    pub(super) scenes_json: Value,
}

pub(super) struct GalgameSceneRegenerationStageOutputs {
    pub(super) lines_draft_json: Value,
    pub(super) lines_staged_json: Value,
    pub(super) visual_plan_json: Value,
    pub(super) branch_plan_json: Value,
    pub(super) staging: GalgameCharacterStaging,
    pub(super) visual_plan: GalgameVisualPlan,
    pub(super) branch_plan: GalgameBranchDesign,
}
