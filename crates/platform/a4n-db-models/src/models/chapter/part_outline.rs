//! 分部大纲实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "part_outline")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,

    pub part_number: i32,
    pub title: String,
    pub start_chapter: i32,
    pub end_chapter: i32,

    #[sea_orm(column_type = "Text")]
    pub summary: Option<String>,
    pub theme: Option<String>,

    #[sea_orm(column_type = "Json")]
    pub key_events: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub character_focus: Option<serde_json::Value>,

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
