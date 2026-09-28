//! Editable domain artifacts, separate from immutable model-call checkpoints.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "media_authoring_document")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub source_snapshot_id: i64,
    pub source_chapter_id: Option<i64>,
    pub kind: String,
    pub version: i32,
    pub revision: i32,
    pub status: String,
    #[sea_orm(column_type = "JsonBinary")]
    pub inputs_json: serde_json::Value,
    #[sea_orm(column_type = "JsonBinary")]
    pub document_json: serde_json::Value,
    pub document_sha256: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
