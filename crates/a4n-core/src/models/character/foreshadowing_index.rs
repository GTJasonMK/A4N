//! 伏笔索引实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "foreshadowing_index")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,

    /// 伏笔描述
    #[sea_orm(column_type = "Text")]
    pub description: String,

    /// 埋设章节
    pub setup_chapter: i32,

    /// 揭示章节
    pub payoff_chapter: Option<i32>,

    /// 规范值由 `a4n_domain::character::ForeshadowingStatus` 定义。
    pub status: String,

    /// 关联角色
    #[sea_orm(column_type = "Json")]
    pub related_characters: Option<serde_json::Value>,

    /// 重要程度（1~5）
    pub importance: i32,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
