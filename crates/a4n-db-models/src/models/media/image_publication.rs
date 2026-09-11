//! 可恢复的图片发布意图实体。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "image_publication")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub publication_id: String,
    pub project_id: i64,
    pub requested_by_user_id: i64,
    pub staging_path: String,
    pub final_path: String,
    pub image_type: String,
    #[sea_orm(column_type = "Text")]
    pub prompt: String,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub expected_file_size: i64,
    pub expected_sha256: String,
    pub file_size: Option<i64>,
    pub file_sha256: Option<String>,
    pub chapter_id: Option<i64>,
    pub panel_id: Option<String>,
    #[sea_orm(column_type = "JsonBinary")]
    pub metadata_json: Option<serde_json::Value>,
    pub status: String,
    pub version: i64,
    pub image_id: Option<i64>,
    #[sea_orm(column_type = "Text")]
    pub last_error: Option<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
