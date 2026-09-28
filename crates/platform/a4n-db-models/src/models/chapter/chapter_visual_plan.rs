//! 章节视觉编排方案实体。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chapter_visual_plan")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub source_revision_key: String,
    pub source_content_sha256: String,
    pub contract_version: String,
    pub plan_version: i32,
    pub revision: i32,
    pub status: String,
    pub authoring_mode: String,
    pub plan_sha256: String,
    #[sea_orm(column_type = "JsonBinary")]
    pub plan_json: serde_json::Value,
    pub author_user_id: i64,
    pub confirmed_at: Option<DateTime>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
