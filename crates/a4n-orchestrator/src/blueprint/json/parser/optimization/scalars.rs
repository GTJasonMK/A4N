use a4n_core::blueprint::{ParsedBlueprintCharacter, ParsedBlueprintRelationship};
use a4n_core::error::AppError;
use serde_json::Value;

use super::super::super::fields::optional_string;

pub(super) fn parse_optimization_scalars(
    value: &Value,
) -> Result<(Option<String>, Option<String>), AppError> {
    Ok((
        optional_string(value, "style")?,
        optional_string(value, "full_synopsis")?,
    ))
}

pub(super) fn ensure_optimization_has_value(
    style: Option<&String>,
    full_synopsis: Option<&String>,
    world_setting: Option<&Value>,
    characters: Option<&Vec<ParsedBlueprintCharacter>>,
    relationships: Option<&Vec<ParsedBlueprintRelationship>>,
) -> Result<(), AppError> {
    if style.is_none()
        && full_synopsis.is_none()
        && world_setting.is_none()
        && characters.is_none()
        && relationships.is_none()
    {
        return Err(AppError::InvalidArgument(
            "蓝图优化结果必须包含至少一个非空可应用字段".to_string(),
        ));
    }
    Ok(())
}
