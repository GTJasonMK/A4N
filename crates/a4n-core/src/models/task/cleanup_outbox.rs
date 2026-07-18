//! 项目删除后的持久化清理意图。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "project_cleanup_outbox")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub dedupe_key: String,
    pub cleanup_kind: String,
    pub project_id: i64,
    #[sea_orm(column_type = "JsonBinary")]
    pub payload_json: serde_json::Value,
    pub status: String,
    pub attempt_count: i32,
    pub next_attempt_at: DateTime,
    pub lease_token: Option<String>,
    pub lease_expires_at: Option<DateTime>,
    #[sea_orm(column_type = "Text")]
    pub last_error: Option<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub completed_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
