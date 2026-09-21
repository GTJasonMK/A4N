//! 故事蓝图实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "novel_blueprint")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 一对一关联 novel_project
    pub project_id: i64,

    pub title: String,
    pub genre: String,
    pub style: Option<String>,
    pub tone: Option<String>,
    pub target_audience: Option<String>,

    #[sea_orm(column_type = "Text")]
    pub one_sentence_summary: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub full_synopsis: Option<String>,

    /// 结构化世界观
    #[sea_orm(column_type = "Json")]
    pub world_setting: Option<serde_json::Value>,

    /// 当前版本号（semver）
    pub current_version: String,

    pub needs_part_outlines: bool,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,

    #[sea_orm(column_type = "Text")]
    pub avatar_svg: Option<String>,
    pub avatar_animal: Option<String>,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::novel_project::Entity",
        from = "Column::ProjectId",
        to = "super::novel_project::Column::Id"
    )]
    NovelProject,
    #[sea_orm(has_many = "super::blueprint_character::Entity")]
    BlueprintCharacter,
    #[sea_orm(has_many = "super::blueprint_relationship::Entity")]
    BlueprintRelationship,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::novel_project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NovelProject.def()
    }
}

impl Related<super::blueprint_character::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BlueprintCharacter.def()
    }
}

impl Related<super::blueprint_relationship::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BlueprintRelationship.def()
    }
}
