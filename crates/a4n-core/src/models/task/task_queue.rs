//! 任务队列表实体【新增】

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "task_queue")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// 任务类型
    pub task_type: String,

    /// 优先级：high / medium / low
    pub priority: String,

    /// 状态：pending / running / completed / failed / cancelled
    pub status: String,

    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub workflow_run_id: Option<i64>,

    /// 任务参数（JSON）
    #[sea_orm(column_type = "Json")]
    pub payload: Option<serde_json::Value>,

    /// 进度信息（JSON）
    #[sea_orm(column_type = "Json")]
    pub progress: Option<serde_json::Value>,

    /// 错误信息
    pub error_message: Option<String>,

    /// DAG ID（如果属于 DAG 图）
    pub dag_id: Option<String>,

    pub created_at: chrono::NaiveDateTime,
    pub started_at: Option<chrono::NaiveDateTime>,
    pub completed_at: Option<chrono::NaiveDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::task_dependency::Entity")]
    TaskDependency,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::task_dependency::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TaskDependency.def()
    }
}
