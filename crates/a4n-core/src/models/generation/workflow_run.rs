//! 持久化工作流运行记录。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_run")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub business_key: String,
    pub workflow_kind: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub scope_table: Option<String>,
    pub scope_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub mode: Option<String>,
    #[sea_orm(column_type = "Json")]
    pub input_fingerprint_json: Option<serde_json::Value>,
    pub status: String,
    pub current_step_key: Option<String>,
    #[sea_orm(column_type = "Json")]
    pub progress_json: Option<serde_json::Value>,
    pub active_task_id: Option<String>,
    pub can_resume: bool,
    pub resume_block_reason: Option<String>,
    pub error_message: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub completed_at: Option<chrono::NaiveDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::workflow_checkpoint::Entity")]
    WorkflowCheckpoint,
    #[sea_orm(has_many = "super::workflow_progress_event::Entity")]
    WorkflowProgressEvent,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::workflow_checkpoint::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkflowCheckpoint.def()
    }
}

impl Related<super::workflow_progress_event::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkflowProgressEvent.def()
    }
}
