//! 场景内实体状态变化实体。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "scene_entity_state")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub scene_profile_id: i64,
    pub occurrence_id: i64,
    pub chapter_number: i32,

    pub entity_type: String,
    pub entity_id: Option<i64>,
    pub entity_name_snapshot: String,
    pub state_key: String,

    #[sea_orm(column_type = "Json")]
    pub state_before_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub state_after_json: serde_json::Value,

    #[sea_orm(column_type = "Text")]
    pub position_description: Option<String>,

    #[sea_orm(column_type = "Text")]
    pub action_description: String,

    #[sea_orm(column_type = "Json")]
    pub source_span_ids_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub evidence_json: serde_json::Value,

    pub created_at: chrono::NaiveDateTime,
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
        belongs_to = "super::scene_occurrence::Entity",
        from = "Column::OccurrenceId",
        to = "super::scene_occurrence::Column::Id"
    )]
    SceneOccurrence,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::scene_profile::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SceneProfile.def()
    }
}

impl Related<super::scene_occurrence::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SceneOccurrence.def()
    }
}
