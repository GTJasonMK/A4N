use super::super::{
    LINE_TYPE_DIALOGUE, LINE_TYPE_MONOLOGUE, LINE_TYPE_NARRATION, LINE_TYPE_STAGE_DIRECTION,
};

mod scene_script;
mod staging;
mod visual_branch;

pub use scene_script::{
    GalgamePipelineBeatRefV3, GalgamePipelineBranchSceneLineDraft, GalgamePipelineLineDraft,
    GalgamePipelineScene, GalgamePipelineSceneLineDraft, GalgamePipelineShootingBeat,
    GalgameScriptAdaptation, GalgameShootingScriptProjection,
};
pub(in crate::galgame::pipeline_json) use scene_script::{
    LineDraftRaw, ScriptAdaptationRaw, ShootingScriptProjectionRaw,
};
pub(in crate::galgame::pipeline_json) use staging::{CharacterStagingRaw, LineStagedRaw};
pub use staging::{
    GalgameCharacterStaging, GalgameFacingV2, GalgameLinePresentationDocumentV2,
    GalgameLinePresentationV2, GalgamePipelineBranchSceneLineStaged, GalgamePipelineLineStaged,
    GalgamePipelineSceneLineStaged, GalgameStageLayerV2, GalgameStagePositionV2,
    GalgameVisibleCastMemberV2,
};
pub(in crate::galgame::pipeline_json) use visual_branch::{BranchDesignRaw, VisualPlanRaw};
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
pub(super) const CG_IMPORTANCE: &[&str] = &["normal", "high", "critical"];
