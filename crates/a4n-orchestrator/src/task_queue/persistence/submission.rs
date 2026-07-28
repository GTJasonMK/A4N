use a4n_core::error::AppError;
use a4n_queue::{
    priority::{TaskInfo, TaskKind, TaskPriority, TaskQueue},
    queue::{TaskFamilyContext, TaskWorkflowContext},
};

use super::super::{PersistentTaskQueue, queue_db_error};

mod record;
#[cfg(test)]
mod tests;

use record::task_queue_record_input;

impl PersistentTaskQueue {
    /// 登记已由同一数据库事务创建的 task_queue row；不再次写库或生成 id。
    pub fn register_persisted(&self, task: TaskInfo) -> Result<(), AppError> {
        self.inner.submit_persisted(task)
    }

    async fn persist_task(
        &self,
        task_id: &str,
        task_type: &TaskKind,
        priority: &TaskPriority,
        project_id: Option<i64>,
        context: Option<&TaskFamilyContext>,
        workflow: Option<&TaskWorkflowContext>,
    ) -> Result<(), AppError> {
        a4n_db::queue::insert_task_queue_record_if_absent(
            self.db.as_ref(),
            task_queue_record_input(task_id, task_type, priority, project_id, context, workflow),
        )
        .await
        .map_err(queue_db_error)
    }

    pub(in crate::task_queue) async fn submit_internal(
        &self,
        task_type: TaskKind,
        priority: TaskPriority,
        project_id: Option<i64>,
        context: Option<&TaskFamilyContext>,
        workflow: Option<&TaskWorkflowContext>,
    ) -> Result<String, AppError> {
        let queue_project_id = context.map_or(project_id, |ctx| Some(ctx.project_id));
        let task_id = self
            .inner
            .submit(task_type, priority, queue_project_id)
            .await?;
        self.persist_task(
            &task_id, &task_type, &priority, project_id, context, workflow,
        )
        .await?;
        Ok(task_id)
    }

    pub async fn submit_with_workflow_context(
        &self,
        task_type: TaskKind,
        priority: TaskPriority,
        project_id: Option<i64>,
        workflow: TaskWorkflowContext,
    ) -> Result<String, AppError> {
        self.submit_internal(task_type, priority, project_id, None, Some(&workflow))
            .await
    }
}
