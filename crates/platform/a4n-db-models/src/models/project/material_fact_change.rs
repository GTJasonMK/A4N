use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "material_fact_change")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub workflow_run_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub change_key: String,
    pub material_kind: String,
    pub target_id: Option<i64>,
    pub field_path: String,
    pub operation: String,
    pub before_json: Json,
    pub after_json: Json,
    pub source_refs_json: Json,
    pub evidence_json: Json,
    pub reason: String,
    pub status: String,
    pub created_at: DateTime,
    pub invalidated_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
