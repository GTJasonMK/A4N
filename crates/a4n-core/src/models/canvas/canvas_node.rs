//! Editable node in a canvas graph.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "canvas_node")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub graph_id: i64,
    pub node_kind: String,
    pub ref_type: Option<String>,
    pub ref_id: Option<i64>,
    #[sea_orm(column_type = "Json")]
    pub config_json: serde_json::Value,
    #[sea_orm(column_type = "Json")]
    pub position_json: serde_json::Value,
    pub state: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::canvas_graph::Entity",
        from = "Column::GraphId",
        to = "super::canvas_graph::Column::Id"
    )]
    CanvasGraph,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::canvas_graph::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CanvasGraph.def()
    }
}
