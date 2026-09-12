use a4n_error::AppError;
use a4n_queue::{
    TaskWorkflowContext,
    priority::{TaskPermit, TaskStatus},
};
use tokio_util::sync::CancellationToken;

use super::super::{PersistentTaskQueue, RunningTask, SubmittedTaskGuard};

impl PersistentTaskQueue {
    /// Generation lifecycle persistence owns every database status write.
    /// The queue contributes only the in-memory execution permit.
    pub async fn acquire_generation_task(
        &self,
        task_id: &str,
    ) -> Result<Option<TaskPermit>, AppError> {
        self.inner
            .acquire_task_with_cancel_non_reentrant(task_id, CancellationToken::new())
            .await
    }

    pub(in crate::task_queue) async fn acquire_task_with_cancel(
        &self,
        task_id: &str,
        cancel_token: CancellationToken,
    ) -> Result<Option<TaskPermit>, AppError> {
        match self
            .inner
            .acquire_task_with_cancel(task_id, cancel_token)
            .await?
        {
            Some(permit) => Ok(Some(self.mark_permit_running(permit).await?)),
            None => Ok(None),
        }
    }

    pub(crate) async fn start_task_with_cancel(
        &self,
        task_type: a4n_queue::priority::TaskKind,
        priority: a4n_queue::priority::TaskPriority,
        project_id: Option<i64>,
        workflow: Option<TaskWorkflowContext>,
        cancel_token: CancellationToken,
    ) -> Result<RunningTask, AppError> {
        self.start_task_with_limit(
            task_type,
            priority,
            project_id,
            workflow,
            cancel_token,
            None,
        )
        .await
    }

    pub(crate) async fn start_task_with_limit(
        &self,
        task_type: a4n_queue::priority::TaskKind,
        priority: a4n_queue::priority::TaskPriority,
        project_id: Option<i64>,
        workflow: Option<TaskWorkflowContext>,
        cancel_token: CancellationToken,
        llm_limit: Option<a4n_domain::task_queue::LlmConcurrencyLimit>,
    ) -> Result<RunningTask, AppError> {
        let task_id = self
            .submit_internal(task_type, priority, project_id, None, workflow.as_ref())
            .await?;
        let mut guard = self.submitted_task_guard(task_id.clone());
        let permit = match llm_limit {
            Some(limit) => {
                self.inner
                    .acquire_llm_call_with_cancel(&task_id, cancel_token, limit)
                    .await?
            }
            None => {
                self.inner
                    .acquire_task_with_cancel(&task_id, cancel_token)
                    .await?
            }
        };
        let Some(permit) = permit else {
            return self.cancel_submitted_task(&task_id, guard).await;
        };
        let permit = self.mark_permit_running(permit).await?;
        guard.disarm();
        Ok(RunningTask::new(self.db.clone(), permit))
    }

    pub(crate) fn signal_generation_cancel(&self, task_id: &str) -> Result<bool, AppError> {
        self.inner.signal_task_cancel(task_id)
    }

    pub(crate) fn project_generation_terminal(
        &self,
        task_id: &str,
        status: TaskStatus,
    ) -> Result<(), AppError> {
        self.inner.project_task_terminal(task_id, status)
    }

    pub(in crate::task_queue) fn submitted_task_guard(
        &self,
        task_id: String,
    ) -> SubmittedTaskGuard {
        SubmittedTaskGuard::new(self.inner.clone(), self.db.clone(), task_id)
    }
}
