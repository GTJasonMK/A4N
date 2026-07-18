use super::{PROJECT_KIND_TEXT, SOURCE_MODE_SELF, SOURCE_SYNC_CURRENT, normalize_project_type};
use a4n_core::{
    models::{
        chapter::{chapter, chapter_outline, part_outline},
        project::{
            blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
            novel_project,
        },
    },
    state::validator::StateValidator,
};
use chrono::{NaiveDateTime, Utc};
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseConnection, Set, TransactionTrait};
use std::collections::HashMap;

mod blueprint;
mod import;
mod outlines;
mod types;
mod validation;

pub use import::import_text_project_record;
pub use types::*;
pub use validation::validate_import_text_project_chapter_bounds;

#[cfg(test)]
mod tests;
