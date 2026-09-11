//! 持久化工作流进度事件。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_progress_event")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub workflow_run_id: i64,
    pub task_id: Option<String>,
    pub event_index: i64,
    pub current: i32,
    pub total: i32,
    pub message: String,
    pub percent: f32,
    #[sea_orm(column_type = "Json")]
    pub meta_json: Option<serde_json::Value>,
    pub created_at: chrono::NaiveDateTime,
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
