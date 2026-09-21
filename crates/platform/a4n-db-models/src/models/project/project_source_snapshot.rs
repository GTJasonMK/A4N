//! 项目源文本快照实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "project_source_snapshot")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub root_project_id: i64,
    pub source_version_hash: String,
    pub blueprint_version_id: Option<i64>,
    #[sea_orm(column_type = "Json")]
    pub chapter_scope_json: serde_json::Value,
    #[sea_orm(column_type = "Json")]
    pub chapter_hashes_json: serde_json::Value,
    #[sea_orm(column_type = "Json")]
    pub profile_hashes_json: serde_json::Value,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
