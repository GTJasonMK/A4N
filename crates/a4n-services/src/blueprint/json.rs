//! Strict JSON contract for blueprint generation responses.

mod entities;
mod fields;
mod parser;
mod types;
mod version;
mod world;

#[cfg(test)]
mod tests;

const MIN_TOTAL_CHAPTERS: i32 = 5;
const MAX_TOTAL_CHAPTERS: i32 = 10_000;
pub(crate) const PART_OUTLINE_THRESHOLD: i32 = 50;
pub(crate) const CHAPTERS_PER_PART: i32 = 25;

pub(super) use parser::{
    build_generation_payload, parse_blueprint_json, parse_blueprint_optimization_json,
    validate_characters_payload, validate_characters_stage, validate_relationships_payload,
    validate_relationships_stage, validate_story_core_stage, validate_world_setting_payload,
    validate_world_stage,
};
pub(super) use types::{ParsedBlueprintCharacter, ParsedBlueprintRelationship};
pub(super) use version::increment_version;

pub(super) fn validate_total_chapter_count(
    value: i32,
    context: &str,
) -> Result<(), a4n_core::error::AppError> {
    fields::validate_chapter_count(value, context)
}
