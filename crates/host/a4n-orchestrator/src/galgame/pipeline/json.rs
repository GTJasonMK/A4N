mod branch;
mod contracts;
mod lines;
mod rules;
mod scene_script;
mod staging;
mod visual;

pub use branch::branch_design_from_value;
pub use contracts::{
    GalgameAdaptationProjection, GalgameBranchDesign, GalgameCharacterStaging,
    GalgamePipelineBackgroundPlan, GalgamePipelineBranchSceneLineStaged, GalgamePipelineChoicePlan,
    GalgamePipelineEventCgPlan, GalgamePipelineLineDraft, GalgamePipelineLineStaged,
    GalgamePipelineScene, GalgameScriptAdaptation, GalgameVisualPlan,
};
#[cfg(test)]
pub use contracts::{
    GalgameLinePresentationV2, GalgamePipelineSceneLineDraft, GalgamePipelineSceneLineStaged,
};
pub(in crate::galgame) use lines::validate_staged_presentation;
pub use scene_script::{adaptation_projection_from_value, script_adaptation_from_value};
pub use staging::character_staging_from_value;
pub use visual::visual_plan_from_value;

#[cfg(test)]
mod tests;
