use a4n_core::blueprint::ParsedBlueprintRelationship;
use a4n_core::error::AppError;
use serde_json::Value;

use super::super::super::fields::{
    reject_keys, reject_unknown_keys, required_object_with_keys, required_strength,
    required_string, validate_enum,
};
pub(super) fn parse_relationship_entry(
    (index, item): (usize, &Value),
) -> Result<ParsedBlueprintRelationship, AppError> {
    let context = format!("blueprint.relationships[{index}]");
    let object = item
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context} 必须是 JSON 对象")))?;
    reject_keys(
        object,
        &["character_from", "character_to", "relation_type"],
        &context,
    )?;
    reject_unknown_keys(
        object,
        &[
            "relationship_type",
            "character_a",
            "character_b",
            "description",
            "strength",
            "evolution",
        ],
        &context,
    )?;
    let relationship_type = required_string(item, "relationship_type", &context)?;
    validate_enum(
        &relationship_type,
        "relationship_type",
        &context,
        &[
            "family", "friend", "enemy", "lover", "rival", "mentor", "other",
        ],
    )?;
    Ok(ParsedBlueprintRelationship {
        relationship_type,
        character_a: required_string(item, "character_a", &context)?,
        character_b: required_string(item, "character_b", &context)?,
        description: required_string(item, "description", &context)?,
        strength: Some(required_strength(item, "strength", &context)?),
        evolution: Some(required_object_with_keys(
            item,
            "evolution",
            &context,
            &[
                "start",
                "tension",
                "turning_points",
                "hidden_truth",
                "story_function",
                "end",
            ],
        )?),
    })
}
