//! 源文本同步运行实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "source_sync_run")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub derived_project_id: i64,
    pub root_project_id: i64,
    pub from_snapshot_id: i64,
    pub to_snapshot_id: i64,
    pub mode: String,
    pub status: String,
    #[sea_orm(column_type = "Json")]
    pub summary_json: serde_json::Value,
    #[sea_orm(column_type = "Text")]
    pub error_message: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
