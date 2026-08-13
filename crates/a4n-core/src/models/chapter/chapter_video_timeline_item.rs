//! 章节视频时间线项实体。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chapter_video_timeline_item")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub timeline_id: i64,
    pub position: Option<i32>,
    /// native / inserted
    pub item_kind: String,
    /// active / deleted / needs_reanchor
    pub status: String,
    pub native_source_shot_id: Option<String>,
    pub native_segment_index_in_shot: Option<i32>,
    pub insert_plan_prompt_id: Option<i64>,
    pub workflow_run_id: Option<i64>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
