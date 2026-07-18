use a4n_core::error::AppError;
use serde_json::Value;

use super::generation_contract::{
    CHARACTERS_STAGE_KEYS, RELATIONSHIPS_STAGE_KEYS, STORY_CORE_STAGE_KEYS, WORLD_STAGE_KEYS,
};
use super::json::{
    validate_characters_stage, validate_relationships_stage, validate_story_core_stage,
    validate_world_stage,
};

mod assembly;
mod checkpoint;
mod completion;
pub(super) mod contracts;
mod execution;
mod preview;
mod stage;
mod trace;

pub(crate) use contracts::BLUEPRINT_GENERATION_STAGES;
pub(super) use preview::preview_generate_requests;
