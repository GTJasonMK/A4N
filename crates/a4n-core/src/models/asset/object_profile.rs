//! 物体档案实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "object_profile")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,

    #[serde(default = "crate::profile_ownership::legacy_unknown_profile_ownership")]
    pub ownership_kind: String,

    pub name: String,
    pub normalized_name: String,

    #[sea_orm(column_type = "Json")]
    pub aliases_json: serde_json::Value,

    pub object_type: String,
    pub importance: String,

    #[sea_orm(column_type = "Text")]
    pub visual_description: String,

    #[sea_orm(column_type = "Text")]
    pub material_and_state: String,

    #[sea_orm(column_type = "Text")]
    pub story_usage: String,

    #[sea_orm(column_type = "Text")]
    pub visual_usage: String,

    #[sea_orm(column_type = "Text")]
    pub generation_usage: String,

    #[sea_orm(column_type = "Text")]
    pub location_context: Option<String>,

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
