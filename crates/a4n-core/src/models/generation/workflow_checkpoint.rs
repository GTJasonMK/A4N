//! 持久化工作流检查点。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_checkpoint")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub workflow_run_id: i64,
    pub step_key: String,
    pub step_order: i32,
    pub status: String,
    pub input_fingerprint_sha256: Option<String>,
    #[sea_orm(column_type = "Json")]
    pub input_fingerprint_json: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub checkpoint_json: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub artifacts_json: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub generation_run_ids_json: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub completed_at: Option<chrono::NaiveDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::workflow_run::Entity",
        from = "Column::WorkflowRunId",
        to = "super::workflow_run::Column::Id"
    )]
    WorkflowRun,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::workflow_run::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkflowRun.def()
    }
}
