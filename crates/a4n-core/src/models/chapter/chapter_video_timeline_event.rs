//! 章节视频时间线变更事件实体。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chapter_video_timeline_event")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub timeline_id: i64,
    pub revision: i64,
    pub operation: String,
    pub item_id: Option<i64>,
    #[sea_orm(column_type = "Json")]
    pub payload_json: serde_json::Value,
    pub actor_user_id: Option<i64>,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
