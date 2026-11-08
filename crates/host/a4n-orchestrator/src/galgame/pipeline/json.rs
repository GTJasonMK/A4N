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

pub(crate) fn output_schema(prompt_name: &str) -> Result<serde_json::Value, a4n_error::AppError> {
    use a4n_domain::json_contract::typed_output_schema;
    Ok(match prompt_name {
        "galgame_script_adaptation" => typed_output_schema::<contracts::ScriptAdaptationRaw>(),
        "galgame_character_staging" => typed_output_schema::<contracts::CharacterStagingRaw>(),
        "galgame_visual_planning" => typed_output_schema::<contracts::VisualPlanRaw>(),
        "galgame_branch_design" => typed_output_schema::<contracts::BranchDesignRaw>(),
        _ => {
            return Err(a4n_error::AppError::InvalidArgument(format!(
                "未知 Galgame 输出合同：{prompt_name}"
            )));
        }
    })
}

#[cfg(test)]
#[test]
fn model_contract_matrix() {
    for name in [
        "galgame_script_adaptation",
        "galgame_character_staging",
        "galgame_visual_planning",
        "galgame_branch_design",
    ] {
        a4n_test_support::model_contract::emit_schema(name, output_schema(name).unwrap());
    }
}

#[cfg(test)]
mod tests;
