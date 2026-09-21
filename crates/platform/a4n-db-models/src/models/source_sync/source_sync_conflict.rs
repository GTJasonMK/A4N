//! 源文本同步冲突实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "source_sync_conflict")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub sync_run_id: i64,
    pub derived_project_id: i64,
    pub root_project_id: i64,
    pub from_snapshot_id: i64,
    pub to_snapshot_id: i64,
    pub conflict_scope: String,
    pub target_kind: String,
    pub target_id: i64,
    pub source_change_kind: String,
    pub local_state_kind: String,
    pub severity: String,
    pub status: String,
    pub resolution_strategy: Option<String>,
    #[sea_orm(column_type = "Json")]
    pub source_summary_json: serde_json::Value,
    #[sea_orm(column_type = "Json")]
    pub local_summary_json: serde_json::Value,
    #[sea_orm(column_type = "Json")]
    pub affected_refs_json: serde_json::Value,
    #[sea_orm(column_type = "Json")]
    pub decision_json: Option<serde_json::Value>,
    pub created_at: chrono::NaiveDateTime,
    pub resolved_at: Option<chrono::NaiveDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
