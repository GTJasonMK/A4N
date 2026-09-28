use super::super::super::{
    LINE_TYPE_DIALOGUE, LINE_TYPE_MONOLOGUE, LINE_TYPE_NARRATION, LINE_TYPE_STAGE_DIRECTION,
};

mod scene_script;
mod staging;
mod visual_branch;

pub(super) use scene_script::{AdaptationProjectionRaw, LineDraftRaw, ScriptAdaptationRaw};
pub use scene_script::{
    GalgameAdaptationProjection, GalgamePipelineBeatRefV4, GalgamePipelineBranchSceneLineDraft,
    GalgamePipelineLineDraft, GalgamePipelineScene, GalgamePipelineSceneLineDraft,
    GalgameScriptAdaptation,
};
pub(super) use staging::{CharacterStagingRaw, LineStagedRaw};
pub use staging::{
    GalgameCharacterStaging, GalgameLinePresentationV2, GalgamePipelineBranchSceneLineStaged,
    GalgamePipelineLineStaged, GalgamePipelineSceneLineStaged,
};
pub(super) use visual_branch::{BranchDesignRaw, VisualPlanRaw};
pub use visual_branch::{
    GalgameBranchDesign, GalgamePipelineBackgroundPlan, GalgamePipelineChoiceOptionPlan,
    GalgamePipelineChoicePlan, GalgamePipelineEventCgPlan, GalgameVisualPlan,
};

pub(super) const SCRIPT_LINE_TYPES: &[&str] = &[
    LINE_TYPE_DIALOGUE,
    LINE_TYPE_NARRATION,
    LINE_TYPE_MONOLOGUE,
    LINE_TYPE_STAGE_DIRECTION,
];
