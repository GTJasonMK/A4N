use super::{
    BlueprintCharacterDraft, BlueprintRelationshipDraft, BlueprintSyncError,
    ProjectStatusHistoryInput, ProjectStatusWriteError, insert_project_status_history,
    sync_blueprint_characters, sync_blueprint_relationships, validate_positive_i64,
};
use a4n_core::{
    models::project::{blueprint_version, novel_blueprint, novel_project},
    state::validator::parse_project_status,
};
use a4n_domain::project::ProjectStatus;
use chrono::{NaiveDateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, Set, TransactionTrait,
};
use serde_json::Value;

mod fields;
mod project_sync;
mod restore;
mod snapshots;
mod types;

pub use restore::*;
pub use types::*;

pub fn validate_blueprint_version_snapshot(data: &Value) -> Result<(), BlueprintRestoreError> {
    snapshots::validate_blueprint_version_snapshot(data)
}

#[cfg(test)]
mod tests;
