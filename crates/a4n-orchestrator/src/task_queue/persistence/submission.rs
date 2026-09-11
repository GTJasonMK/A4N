use a4n_domain::task_queue::PersistTaskQueueRecordInput;
use a4n_error::AppError;
use a4n_queue::{
    TaskFamilyContext, TaskWorkflowContext,
    priority::{TaskInfo, TaskKind, TaskPriority, TaskStatus},
};
use chrono::Utc;
use uuid::Uuid;

use super::super::{PersistentTaskQueue, queue_db_error};

#[cfg(test)]
mod tests;

impl PersistentTaskQueue {
    /// 登记已由同一数据库事务创建的 task_queue row；不再次写库或生成 id。
    pub fn register_persisted(&self, task: TaskInfo) -> Result<(), AppError> {
        self.inner.submit_persisted(task)
    }

    pub(in crate::task_queue) async fn submit_internal(
        &self,
        task_type: TaskKind,
        priority: TaskPriority,
        project_id: Option<i64>,
        context: Option<&TaskFamilyContext>,
        workflow: Option<&TaskWorkflowContext>,
    ) -> Result<String, AppError> {
        let task_id = Uuid::new_v4().to_string();
        let record = PersistTaskQueueRecordInput {
            task_id: task_id.clone(),
            task_type: task_type.as_str().to_string(),
            priority: priority.as_str().to_string(),
            project_id: context.map_or(project_id, |context| Some(context.project_id)),
            root_project_id: context.map(|context| context.root_project_id),
            source_snapshot_id: context.and_then(|context| context.source_snapshot_id),
            task_scope: context.map(|context| context.task_scope.clone()),
            workflow_run_id: workflow.map(|context| context.workflow_run_id),
        };
        let now = Utc::now().naive_utc();
        let task = TaskInfo {
            id: task_id.clone(),
            task_type,
            priority,
            status: TaskStatus::Pending,
            project_id: record.project_id,
            root_project_id: record.root_project_id,
            source_snapshot_id: record.source_snapshot_id,
            task_scope: record.task_scope.clone(),
            progress: None,
            created_at: now,
            started_at: None,
            completed_at: None,
        };
        // Persist first. A crash between these two operations is recoverable
        // by the existing startup scan; submitting to memory first could let a
        // worker execute a task whose package admission transaction later lost
        // the race with a disable operation.
        a4n_db::queue::insert_task_queue_record(self.db.as_ref(), record)
            .await
            .map_err(queue_db_error)?;
        if let Err(error) = self.inner.submit_persisted(task) {
            a4n_db::queue::update_task_status(
                self.db.as_ref(),
                &task_id,
                "cancelled",
                Some("内存队列登记失败，避免遗留未执行任务"),
            )
            .await
            .map_err(queue_db_error)?;
            return Err(error);
        }
        Ok(task_id)
    }
}
