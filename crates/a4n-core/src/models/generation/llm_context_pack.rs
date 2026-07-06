//! LLM 上下文包实体。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "llm_context_pack")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    pub scope_kind: String,
    pub workflow: String,
    pub pack_kind: String,
    #[sea_orm(column_type = "Json")]
    pub source_refs_json: Option<serde_json::Value>,
    pub config_hash: String,
    pub content_sha256: String,
    #[sea_orm(column_type = "Json")]
    pub canonical_content_json: serde_json::Value,
    #[sea_orm(column_type = "Text")]
    pub canonical_content_text: String,
    pub compression_level: String,
    pub token_estimate: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
