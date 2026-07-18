mod contracts;
mod execution;
mod persistence;
mod request;
mod rules;
mod workflow;

pub use execution::discover_character_profiles_for_chapters;
pub(crate) use request::character_discovery_preview_steps;
#[cfg(test)]
pub(super) use rules::{
    select_character_discovery_chapters, validate_discovered_character_chapter_scope,
};
