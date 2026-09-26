//! Galgame scene entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "galgame_scene")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub script_id: i64,
    pub source_snapshot_chapter_id: i64,
    #[sea_orm(column_type = "Text", nullable)]
    pub shooting_scene_identity: Option<String>,
    pub scene_index: i32,
    #[sea_orm(column_type = "Text")]
    pub title: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    #[sea_orm(column_type = "Json")]
    pub participants_json: serde_json::Value,
    #[sea_orm(column_type = "Text", nullable)]
    pub background_note: Option<String>,
    pub entry_kind: String,
    pub next_scene_id: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::galgame_script::Entity",
        from = "Column::ScriptId",
        to = "super::galgame_script::Column::Id"
    )]
    Script,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::galgame_script::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Script.def()
    }
}
