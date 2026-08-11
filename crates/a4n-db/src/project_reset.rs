//! 项目生成态重置与状态回退清理（阶段 11：从 a4n-services::project 迁入）。

use a4n_core::models::project::novel_project;
use a4n_core::state::{
    rollback::{CleanupAction, rollback_actions},
    transitions::is_valid_rollback,
    validator::parse_project_status,
};
use a4n_domain::project::ProjectStatus;
use chrono::{NaiveDateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, Set, TransactionTrait,
};

use crate::character;
use crate::generation;
use crate::project::{
    self as project_db, PROJECT_KIND_GALGAME, PROJECT_KIND_TEXT, ProjectStatusHistoryInput,
    ProjectTextAnchorDeleteScope,
};
use crate::project_delete::{
    GeneratedImageDeleteScope, delete_chapter_generation_outputs, delete_project_profile_scene_data,
};

mod types;
pub use types::*;

mod galgame;
use galgame::reset_galgame_generated_data_in_tx;

mod characters;
use characters::reset_character_profiles_for_project;

mod reset;
pub use reset::*;

mod rollback;
pub use rollback::*;

#[cfg(test)]
mod tests;
