//! 场景档案实体。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "scene_profile")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,

    pub name: String,
    pub normalized_name: String,

    #[sea_orm(column_type = "Json")]
    pub aliases_json: serde_json::Value,

    pub scene_type: String,
    pub importance: String,
    pub parent_scene_id: Option<i64>,

    #[sea_orm(column_type = "Text")]
    pub base_description: String,

    #[sea_orm(column_type = "Text")]
    pub visual_description: String,

    #[sea_orm(column_type = "Text")]
    pub spatial_layout: String,

    #[sea_orm(column_type = "Text")]
    pub atmosphere: String,

    #[sea_orm(column_type = "Json")]
    pub entry_points_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub rules_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub current_state_json: serde_json::Value,

    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,

    #[sea_orm(column_type = "Json")]
    pub evidence_json: serde_json::Value,

    #[sea_orm(column_type = "Text")]
    pub image_prompt_seed: String,

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
