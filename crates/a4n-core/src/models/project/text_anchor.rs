//! 文本锚点实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "text_anchor")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub source_kind: String,
    pub source_id: i64,
    pub paragraph_index: i32,
    pub start_offset: i32,
    pub end_offset: i32,
    #[sea_orm(column_type = "Text")]
    pub quote: String,
    pub quote_hash: String,
    pub source_version_hash: String,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
