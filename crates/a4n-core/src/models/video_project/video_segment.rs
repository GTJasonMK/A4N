//! 视频项目片段实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "video_segment")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub segment_type: String,
    pub order_index: i32,
    #[sea_orm(column_type = "Text")]
    pub title: String,
    pub duration_secs: f32,
    pub source_generation_spec_id: Option<i64>,
    pub result_media_asset_id: Option<i64>,
    pub previous_segment_id: Option<i64>,
    pub next_segment_id: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
