//! Strict JSON contract for blueprint generation responses.

mod entities;
mod fields;
mod parser;
mod world;

#[cfg(test)]
mod tests;

pub(crate) use a4n_db::project::CHAPTERS_PER_PART;

pub use parser::{
    parse_blueprint_json, parse_blueprint_optimization_json, validate_characters_stage,
    validate_relationships_stage, validate_story_core_stage, validate_world_stage,
};

pub(crate) fn validate_sparse_blueprint_sections(
    world_setting: &serde_json::Value,
    characters: &[serde_json::Value],
    relationships: &[serde_json::Value],
) -> Result<(), a4n_core::error::AppError> {
    world::validate_sparse_world_setting(world_setting)?;
    entities::parse_sparse_characters(characters)?;
    entities::parse_sparse_relationships(relationships)?;
    Ok(())
}
