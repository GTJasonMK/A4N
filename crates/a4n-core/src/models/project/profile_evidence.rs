//! 语义档案证据引用实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "profile_evidence")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub profile_kind: String,
    pub profile_id: i64,
    pub text_anchor_id: i64,
    pub evidence_role: String,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
