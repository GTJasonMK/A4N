//! 角色状态索引实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "character_state_index")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,

    /// 角色状态摘要
    #[sea_orm(column_type = "Text")]
    pub state_summary: String,

    /// 关键属性快照
    #[sea_orm(column_type = "Json")]
    pub key_attributes: serde_json::Value,

    /// 情绪状态
    pub emotional_state: Option<String>,

    /// 当前位置
    pub current_location: Option<String>,

    /// 当前目标
    #[sea_orm(column_type = "Text")]
    pub current_goal: Option<String>,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
