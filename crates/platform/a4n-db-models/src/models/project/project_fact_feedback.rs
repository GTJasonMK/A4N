use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "project_fact_feedback")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub project_id: i64,
    pub interval_chapters: i32,
    pub consolidated_sources_json: Json,
    pub last_consolidated_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
