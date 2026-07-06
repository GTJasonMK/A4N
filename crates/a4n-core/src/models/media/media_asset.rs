//! 统一媒体资产实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "media_asset")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub media_kind: String,
    pub origin: String,
    #[sea_orm(column_type = "Text")]
    pub file_path: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub thumbnail_path: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_secs: Option<f32>,
    pub fps: Option<f32>,
    pub file_size: Option<i64>,
    pub checksum: Option<String>,
    pub status: String,
    #[sea_orm(column_type = "Text")]
    pub error_message: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
