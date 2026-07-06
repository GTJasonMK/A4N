use a4n_core::error::AppError;
use serde_json::Value;

use super::fields::{
    reject_keys, reject_unknown_keys, required_object_with_keys, required_strength,
    required_string, validate_enum,
};
use super::types::{ParsedBlueprintCharacter, ParsedBlueprintRelationship};

pub(super) fn parse_characters(items: &[Value]) -> Result<Vec<ParsedBlueprintCharacter>, AppError> {
    if items.len() < 3 {
        return Err(AppError::InvalidArgument(
            "blueprint.characters 至少需要 3 个角色".to_string(),
        ));
    }

    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
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
        })
        .collect()
}

pub(super) fn parse_relationships(
    items: &[Value],
) -> Result<Vec<ParsedBlueprintRelationship>, AppError> {
    if items.is_empty() {
        return Err(AppError::InvalidArgument(
            "blueprint.relationships 不能为空".to_string(),
        ));
    }

    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
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
        })
        .collect()
}
