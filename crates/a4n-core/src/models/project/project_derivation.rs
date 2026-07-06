//! 项目派生关系实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "project_derivation")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub root_project_id: i64,
    pub derived_project_id: i64,
    pub derived_kind: String,
    pub source_snapshot_id: i64,
    #[sea_orm(column_type = "Json")]
    pub chapter_scope_json: serde_json::Value,
    #[sea_orm(column_type = "Json")]
    pub creation_options_json: serde_json::Value,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
