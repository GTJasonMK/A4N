use a4n_core::blueprint::{
    BLUEPRINT_REFINEMENT_PROMPT, ParsedBlueprintCharacter, ParsedBlueprintRelationship,
};
use a4n_core::error::AppError;
use serde_json::Value;

use super::super::super::{
    entities::{parse_characters, parse_relationships},
    world::validate_world_setting,
};

type OptimizationSections = (
    Option<Value>,
    Option<Vec<ParsedBlueprintCharacter>>,
    Option<Vec<ParsedBlueprintRelationship>>,
);

pub(super) fn parse_optimization_sections(value: &Value) -> Result<OptimizationSections, AppError> {
    let world_setting = match value.get("world_setting") {
        Some(value) if !value.is_null() => {
            if !value.is_object() {
                return Err(AppError::InvalidArgument(format!(
                    "{BLUEPRINT_REFINEMENT_PROMPT}.world_setting 必须是 JSON 对象"
                )));
            }
            validate_world_setting(value)?;
            Some(value.clone())
        }
        _ => None,
    };
    let characters = match value.get("characters") {
        Some(value) if !value.is_null() => {
            Some(parse_characters(value.as_array().ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "{BLUEPRINT_REFINEMENT_PROMPT}.characters 必须是数组"
                ))
            })?)?)
        }
        _ => None,
    };
    let relationships = match value.get("relationships") {
        Some(value) if !value.is_null() => {
            Some(parse_relationships(value.as_array().ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "{BLUEPRINT_REFINEMENT_PROMPT}.relationships 必须是数组"
                ))
            })?)?)
        }
        _ => None,
    };
    Ok((world_setting, characters, relationships))
}
