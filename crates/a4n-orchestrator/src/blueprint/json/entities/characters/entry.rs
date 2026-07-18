use a4n_core::blueprint::ParsedBlueprintCharacter;
use a4n_core::error::AppError;
use serde_json::Value;

use super::super::super::fields::{
    reject_keys, reject_unknown_keys, required_object_with_keys, required_string, validate_enum,
};
pub(super) fn parse_character_entry(
    (index, item): (usize, &Value),
) -> Result<ParsedBlueprintCharacter, AppError> {
    let context = format!("blueprint.characters[{index}]");
    let object = item
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context} 必须是 JSON 对象")))?;
    reject_keys(
        object,
        &["role", "identity", "goals", "abilities"],
        &context,
    )?;
    reject_unknown_keys(
        object,
        &[
            "name",
            "character_type",
            "gender",
            "age",
            "appearance",
            "personality",
            "background",
            "motivation",
            "arc_summary",
            "relationship_to_protagonist",
            "arc_stages",
        ],
        &context,
    )?;
    let character_type = required_string(item, "character_type", &context)?;
    validate_enum(
        &character_type,
        "character_type",
        &context,
        &["protagonist", "antagonist", "supporting", "minor"],
    )?;
    Ok(ParsedBlueprintCharacter {
        name: required_string(item, "name", &context)?,
        character_type,
        gender: Some(required_string(item, "gender", &context)?),
        age: Some(required_string(item, "age", &context)?),
        appearance: required_string(item, "appearance", &context)?,
        personality: required_string(item, "personality", &context)?,
        background: required_string(item, "background", &context)?,
        motivation: required_string(item, "motivation", &context)?,
        arc_summary: required_string(item, "arc_summary", &context)?,
        relationship_to_protagonist: required_string(
            item,
            "relationship_to_protagonist",
            &context,
        )?,
        arc_stages: Some(required_object_with_keys(
            item,
            "arc_stages",
            &context,
            &[
                "start",
                "core_wound",
                "desire",
                "need",
                "secret",
                "voice_style",
                "turning_point",
                "end",
            ],
        )?),
    })
}
