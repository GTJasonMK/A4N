//! workflow task lookup port。

use super::*;

#[async_trait::async_trait]
impl WorkflowTaskLookupApplicationPort for OrchestratorContext {
    async fn task_by_id(&self, task_id: &str) -> Result<Option<TaskLookupRecord>, AppError> {
        Ok(self
            .workflow
            .persisted_task_by_id(task_id)
            .await?
            .map(|persisted| TaskLookupRecord {
                task_id: persisted.task.id.clone(),
                project_id: persisted.task.project_id,
                root_project_id: persisted.task.root_project_id,
                workflow_run_id: persisted.workflow_run_id,
                status: persisted.task.status.as_str().to_string(),
            }))
    }

    async fn active_tasks(&self) -> Result<Vec<ActiveTaskLookupRecord>, AppError> {
        self.workflow.active_tasks().await
    }

    async fn workflow_access_by_active_task_id(
        &self,
        task_id: &str,
    ) -> Result<Option<WorkflowTaskAccessRecord>, AppError> {
        Ok(self
            .workflow
            .workflow_run_access_record_by_active_task_id(task_id)
            .await?
            .map(|record| WorkflowTaskAccessRecord {
                workflow_run_id: record.id,
                project_id: record.project_id,
                root_project_id: record.root_project_id,
            }))
    }

    async fn workflow_run_for_resume(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowResumeRecord, AppError> {
        self.workflow
            .workflow_run_resume_record(workflow_run_id)
            .await
    }

    async fn workflow_run_summary(
        &self,
        workflow_run_id: i64,
    ) -> Result<a4n_api_contract::workflow::WorkflowRunSummary, AppError> {
        self.workflow
            .workflow_run_summary(workflow_run_id)
            .await
            .map(workflow_run_summary_to_api)
    }

    async fn workflow_progress_events(
        &self,
        workflow_run_id: i64,
        limit: usize,
    ) -> Result<Vec<a4n_api_contract::workflow::WorkflowProgressEvent>, AppError> {
        self.workflow
            .workflow_progress_events(workflow_run_id, limit)
            .await
            .map(|events| {
                events
                    .into_iter()
                    .map(|event| a4n_api_contract::workflow::WorkflowProgressEvent {
                        id: event.id,
                        current: event.current,
                        total: event.total,
                        message: event.message,
                        meta: event.meta,
                        timestamp: event.timestamp,
                    })
                    .collect()
            })
    }
}
