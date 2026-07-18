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
