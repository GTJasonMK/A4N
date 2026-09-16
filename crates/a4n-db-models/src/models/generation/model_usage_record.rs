//! Durable model-call cost ledger.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "model_usage_record")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub request_id: String,
    pub user_id: Option<i64>,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub workflow_run_id: Option<i64>,
    pub generation_run_id: Option<i64>,
    pub workflow: String,
    pub step_key: String,
    pub call_key: Option<String>,
    pub source: Option<String>,
    pub capability: String,
    pub provider_config_id: Option<i64>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub confidence: String,
    pub currency: String,
    pub amount_micros: Option<i64>,
    #[sea_orm(column_type = "JsonBinary")]
    pub meter_json: serde_json::Value,
    #[sea_orm(column_type = "JsonBinary")]
    pub pricing_snapshot_json: serde_json::Value,
    #[sea_orm(column_type = "Text")]
    pub error_kind: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub error_message: Option<String>,
    pub started_at: chrono::NaiveDateTime,
    pub completed_at: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
