//! 场景位移关系实体。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "scene_transition")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub from_scene_profile_id: Option<i64>,
    pub to_scene_profile_id: i64,
    pub occurrence_id: Option<i64>,
    pub transition_order: i32,
    pub movement_type: String,

    #[sea_orm(column_type = "Text")]
    pub route_description: String,

    pub time_cost: Option<String>,
    pub distance_hint: Option<String>,

    #[sea_orm(column_type = "Json")]
    pub participants_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub object_refs_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub source_span_ids_json: serde_json::Value,

    #[sea_orm(column_type = "Json")]
    pub evidence_json: serde_json::Value,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
