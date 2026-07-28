mod branch;
mod contracts;
mod envelope;
mod lines;
mod rules;
mod scene_script;
mod staging;
mod visual;

pub use branch::branch_design_from_value;
pub use contracts::*;
pub use envelope::parse_galgame_json_object;
pub(in crate::galgame) use lines::validate_staged_presentation;
pub use scene_script::{script_adaptation_from_value, shooting_script_projection_from_value};
pub use staging::character_staging_from_value;
pub use visual::visual_plan_from_value;

pub(crate) fn galgame_llm_output_schema(
    prompt_name: &str,
) -> Result<serde_json::Value, a4n_core::error::AppError> {
    let schema = match prompt_name {
        "galgame_script_adaptation" => schemars::schema_for!(contracts::ScriptAdaptationRaw),
        "galgame_character_staging" => schemars::schema_for!(contracts::CharacterStagingRaw),
        "galgame_visual_planning" => schemars::schema_for!(contracts::VisualPlanRaw),
        "galgame_branch_design" => schemars::schema_for!(contracts::BranchDesignRaw),
        other => {
            return Err(a4n_core::error::AppError::ConfigError(format!(
                "未注册的 Galgame 结构化输出合同: {other}"
            )));
        }
    };
    serde_json::to_value(schema).map_err(a4n_core::error::AppError::Serialization)
}

#[cfg(test)]
mod tests;
