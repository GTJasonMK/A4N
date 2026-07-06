//! Typed edge between canvas nodes.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "canvas_edge")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub graph_id: i64,
    pub source_node_id: i64,
    pub source_port: String,
    pub target_node_id: i64,
    pub target_port: String,
    pub role: String,
    #[sea_orm(column_type = "Json")]
    pub config_json: serde_json::Value,
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
