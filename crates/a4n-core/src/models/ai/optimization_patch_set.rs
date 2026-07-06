use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "optimization_patch_set")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub turn_id: i64,
    pub session_id: i64,
    pub chapter_id: i64,
    pub base_content_hash: String,
    pub status: String,

    #[sea_orm(column_type = "Text")]
    pub summary: String,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::optimization_turn::Entity",
        from = "Column::TurnId",
        to = "super::optimization_turn::Column::Id"
    )]
    OptimizationTurn,
    #[sea_orm(
        belongs_to = "super::optimization_session::Entity",
        from = "Column::SessionId",
        to = "super::optimization_session::Column::Id"
    )]
    OptimizationSession,
    #[sea_orm(has_many = "super::optimization_patch::Entity")]
    Patches,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::optimization_turn::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OptimizationTurn.def()
    }
}

impl Related<super::optimization_session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OptimizationSession.def()
    }
}

impl Related<super::optimization_patch::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Patches.def()
    }
}
