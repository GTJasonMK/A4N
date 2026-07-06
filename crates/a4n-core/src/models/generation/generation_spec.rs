//! 生成意图实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "generation_spec")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_context_kind: Option<String>,
    pub source_snapshot_id: Option<i64>,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub canvas_text_asset_id: Option<i64>,
    pub target_kind: String,
    pub target_entity_kind: String,
    pub target_entity_id: Option<i64>,
    pub status: String,
    pub source_version_hash: String,
    #[sea_orm(column_type = "Json")]
    pub params_json: serde_json::Value,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
