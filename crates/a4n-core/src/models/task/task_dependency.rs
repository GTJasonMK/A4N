//! 任务依赖实体【新增】

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "task_dependency")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 后继任务 ID
    pub task_id: String,

    /// 前置任务 ID
    pub depends_on_task_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::task_queue::Entity",
        from = "Column::TaskId",
        to = "super::task_queue::Column::Id"
    )]
    Task,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::task_queue::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Task.def()
    }
}
