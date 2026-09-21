//! 章节大纲实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const STATUS_DRAFT: &str = "draft";
pub const STATUS_COMPLETED: &str = "completed";

pub fn is_valid_status(status: &str) -> bool {
    matches!(status, STATUS_DRAFT | STATUS_COMPLETED)
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chapter_outline")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub part_id: Option<i64>,

    pub chapter_number: i32,
    pub title: Option<String>,

    #[sea_orm(column_type = "Text")]
    pub summary: Option<String>,

    /// 节拍表条目
    #[sea_orm(column_type = "Json")]
    pub beats: Option<serde_json::Value>,

    /// 本章关键场景
    #[sea_orm(column_type = "Json")]
    pub key_scenes: Option<serde_json::Value>,

    /// 节奏评分
    pub pacing_score: Option<f32>,

    /// 本章伏笔
    #[sea_orm(column_type = "Json")]
    pub foreshadowing_setup: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub foreshadowing_payoff: Option<serde_json::Value>,

    /// 本章涉及的角色
    #[sea_orm(column_type = "Json")]
    pub characters_involved: Option<serde_json::Value>,

    pub word_count_target: Option<i32>,
    pub status: String,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::super::project::novel_project::Entity",
        from = "Column::ProjectId",
        to = "super::super::project::novel_project::Column::Id"
    )]
    NovelProject,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::super::project::novel_project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NovelProject.def()
    }
}
