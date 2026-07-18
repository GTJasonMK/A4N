use a4n_core::blueprint::{ParsedBlueprintCharacter, ParsedBlueprintRelationship};
use a4n_core::error::AppError;
use serde_json::Value;

use super::super::super::{
    entities::{parse_characters, parse_relationships},
    fields::{required_array, required_object_clone},
    world::validate_world_setting,
};

pub(super) fn parse_blueprint_sections(
    raw: &Value,
) -> Result<
    (
        Value,
        Vec<ParsedBlueprintCharacter>,
        Vec<ParsedBlueprintRelationship>,
    ),
    AppError,
> {
    let world_setting = required_object_clone(raw, "world_setting", "blueprint")?;
    validate_world_setting(&world_setting)?;
    let characters = parse_characters(required_array(raw, "characters", "blueprint")?)?;
    let relationships = parse_relationships(required_array(raw, "relationships", "blueprint")?)?;
    Ok((world_setting, characters, relationships))
}
