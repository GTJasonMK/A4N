//! 可恢复生成任务的规范持久检查点。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "generation_task_checkpoint")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub task_id: String,
    pub business_key: String,
    pub attempt: i32,
    pub requested_by_user_id: i64,
    pub task_kind: String,
    pub delivery_mode: String,
    pub batch_policy: Option<String>,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub workflow_run_id: Option<i64>,
    pub generation_run_id: Option<i64>,
    pub target_kind: Option<String>,
    pub target_id: Option<String>,
    pub target_revision: Option<String>,
    pub root_task_id: String,
    pub parent_task_id: Option<String>,
    pub item_key: Option<String>,
    pub retry_of_task_id: Option<String>,
    pub status: String,
    pub checkpoint_revision: i64,
    pub covers_through_sequence: i64,
    pub reserved_through_sequence: i64,
    #[sea_orm(column_type = "JsonBinary")]
    pub public_snapshot_json: Option<serde_json::Value>,
    pub public_snapshot_sha256: Option<String>,
    pub step_key: Option<String>,
    pub call_key: Option<String>,
    pub provider_kind: Option<String>,
    pub provider_model: Option<String>,
    pub provider_job_id: Option<String>,
    pub provider_idempotency_key_sha256: Option<String>,
    pub resume_metadata_version: Option<i32>,
    #[sea_orm(column_type = "JsonBinary")]
    pub execution_checkpoint_json: Option<serde_json::Value>,
    pub execution_checkpoint_sha256: Option<String>,
    pub input_fingerprint_sha256: String,
    pub result_kind: Option<String>,
    pub result_id: Option<String>,
    pub result_revision: Option<String>,
    pub expires_at: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub completed_at: Option<chrono::NaiveDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
