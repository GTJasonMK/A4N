use super::blueprint_sync::{sync_blueprint_characters, sync_blueprint_relationships};
use super::{
    BlueprintCharacterDraft, BlueprintRelationshipDraft, BlueprintSyncError,
    ProjectStatusHistoryInput, ProjectStatusWriteError, insert_project_status_history,
    validate_positive_i64,
};
use a4n_core::{
    models::project::{blueprint_version, novel_blueprint, novel_project},
    state::validator::parse_project_status,
};
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

pub use restore::restore_blueprint_version_record;
pub use types::BlueprintRestoreError;

pub fn validate_blueprint_version_snapshot(data: &Value) -> Result<(), BlueprintRestoreError> {
    snapshots::validate_blueprint_version_snapshot(data)
}

#[cfg(test)]
mod tests;
