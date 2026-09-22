//! 项目生成态重置与状态回退清理（阶段 11：从 a4n-services::project 迁入）。

use a4n_db_models::models::project::novel_project;
use a4n_domain::project::{CleanupAction, ProjectKind, ProjectStatus, rollback_actions};
use chrono::{NaiveDateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, Set, TransactionTrait,
};

use crate::character;
use crate::generation;
use crate::project::delete::{
    GeneratedImageDeleteScope, delete_chapter_generation_outputs, delete_project_profile_scene_data,
};
use crate::project::{self as project_db, PROJECT_KIND_TEXT, ProjectTextAnchorDeleteScope};

mod types;
pub use types::{ProjectResetDbError, ProjectStatusRollbackOutcome};

mod galgame;
use galgame::reset_galgame_generated_data_in_tx;

mod characters;
use characters::reset_character_profiles_for_project;

mod reset;
pub use reset::reset_project_generated_data_record;

mod rollback;
pub use rollback::rollback_project_status_record;

#[cfg(test)]
mod tests;
