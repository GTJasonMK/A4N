//! 场景出场记录实体。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "scene_occurrence")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub scene_profile_id: i64,
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub occurrence_order: i32,

    pub timeline_position: Option<String>,

    #[sea_orm(column_type = "Text")]
    pub summary: String,

    pub mood: Option<String>,
    pub weather_or_lighting: Option<String>,

    #[sea_orm(column_type = "Json")]
    pub source_span_ids_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub evidence_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub characters_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub objects_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub key_events_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub entry_state_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub exit_state_json: serde_json::Value,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::scene_profile::Entity",
        from = "Column::SceneProfileId",
        to = "super::scene_profile::Column::Id"
    )]
    SceneProfile,
    #[sea_orm(
        belongs_to = "super::super::chapter::chapter::Entity",
        from = "Column::ChapterId",
        to = "super::super::chapter::chapter::Column::Id"
    )]
    Chapter,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::scene_profile::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SceneProfile.def()
    }
}

impl Related<super::super::chapter::chapter::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Chapter.def()
    }
}
