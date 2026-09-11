//! Galgame line entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "galgame_line")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub scene_id: i64,
    pub line_index: i32,
    pub line_type: String,
    pub speaker_profile_id: Option<i64>,
    #[sea_orm(column_type = "Text")]
    pub text: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub position: Option<String>,
    #[sea_orm(column_type = "Json")]
    pub presentation_json: serde_json::Value,
    pub text_anchor_id: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::galgame_scene::Entity",
        from = "Column::SceneId",
        to = "super::galgame_scene::Column::Id"
    )]
    Scene,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::galgame_scene::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Scene.def()
    }
}
