//! 源文本快照大纲实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "source_snapshot_outline")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub snapshot_id: i64,
    pub root_project_id: i64,
    pub source_outline_id: i64,
    pub outline_kind: String,
    #[sea_orm(column_type = "Json")]
    pub payload_json: serde_json::Value,
    pub payload_hash: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
