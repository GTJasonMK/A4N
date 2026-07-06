//! Editable canvas graph for video studio workflows.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "canvas_graph")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub graph_kind: String,
    pub version: i32,
    #[sea_orm(column_type = "Json")]
    pub viewport_json: Option<serde_json::Value>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::super::project::novel_project::Entity",
        from = "Column::ProjectId",
        to = "super::super::project::novel_project::Column::Id"
    )]
    NovelProject,
    #[sea_orm(has_many = "super::canvas_node::Entity")]
    CanvasNode,
    #[sea_orm(has_many = "super::canvas_edge::Entity")]
    CanvasEdge,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::super::project::novel_project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NovelProject.def()
    }
}

impl Related<super::canvas_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CanvasNode.def()
    }
}

impl Related<super::canvas_edge::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CanvasEdge.def()
    }
}
