use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "optimization_turn_event")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub turn_id: i64,
    pub seq: i32,
    pub kind: String,
    pub status: String,

    #[sea_orm(column_type = "Text")]
    pub public_text: String,

    #[sea_orm(column_type = "Json")]
    pub payload: Option<serde_json::Value>,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::optimization_turn::Entity",
        from = "Column::TurnId",
        to = "super::optimization_turn::Column::Id"
    )]
    OptimizationTurn,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::optimization_turn::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OptimizationTurn.def()
    }
}
