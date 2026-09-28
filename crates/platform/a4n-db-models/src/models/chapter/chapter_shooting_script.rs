//! 章节结构化拍摄剧本实体。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chapter_shooting_script")]
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
    pub visual_plan_id: Option<i64>,
    pub visual_plan_version: Option<i32>,
    pub visual_plan_sha256: Option<String>,
    pub input_fingerprint_sha256: String,
    pub contract_version: String,
    pub revision: i32,
    pub script_sha256: String,
    #[sea_orm(column_type = "JsonBinary")]
    pub script_json: serde_json::Value,
    pub workflow_run_id: Option<i64>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
