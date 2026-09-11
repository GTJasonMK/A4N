//! 源文本快照章节实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "source_snapshot_chapter")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub snapshot_id: i64,
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub content: String,
    #[sea_orm(column_type = "Text")]
    pub summary: Option<String>,
    #[sea_orm(column_type = "Json")]
    pub analysis_data_json: Option<serde_json::Value>,
    pub content_hash: String,
    pub analysis_hash: Option<String>,
    pub visual_plan_id: Option<i64>,
    pub visual_plan_version: Option<i32>,
    pub visual_plan_sha256: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
