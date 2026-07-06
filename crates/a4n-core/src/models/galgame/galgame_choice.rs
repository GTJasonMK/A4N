//! Galgame choice entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "galgame_choice")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub line_id: i64,
    pub option_index: i32,
    #[sea_orm(column_type = "Text")]
    pub option_text: String,
    pub target_scene_id: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::galgame_line::Entity",
        from = "Column::LineId",
        to = "super::galgame_line::Column::Id"
    )]
    Line,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::galgame_line::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Line.def()
    }
}
