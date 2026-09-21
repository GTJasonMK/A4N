use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "optimization_turn")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub session_id: i64,
    pub agent_turn_id: Option<i64>,

    #[sea_orm(column_type = "Text")]
    pub user_text: String,

    pub status: String,

    #[sea_orm(column_type = "Json")]
    pub permissions_snapshot: Option<serde_json::Value>,

    #[sea_orm(column_type = "Text")]
    pub final_reply: Option<String>,

    #[sea_orm(column_type = "Text")]
    pub error: Option<String>,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::optimization_session::Entity",
        from = "Column::SessionId",
        to = "super::optimization_session::Column::Id"
    )]
    OptimizationSession,
    #[sea_orm(has_many = "super::optimization_turn_event::Entity")]
    Events,
    #[sea_orm(has_many = "super::optimization_patch_set::Entity")]
    PatchSets,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::optimization_session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OptimizationSession.def()
    }
}

impl Related<super::optimization_turn_event::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Events.def()
    }
}

impl Related<super::optimization_patch_set::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PatchSets.def()
    }
}
