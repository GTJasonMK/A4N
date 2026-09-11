//! 源文本同步决策日志实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "source_sync_decision")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub conflict_id: i64,
    pub decided_by: String,
    pub strategy: String,
    #[sea_orm(column_type = "Json")]
    pub decision_payload_json: serde_json::Value,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
