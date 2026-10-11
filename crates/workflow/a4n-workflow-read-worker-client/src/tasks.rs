use super::*;

#[async_trait::async_trait]
impl TaskReadApplicationPort for WorkflowReadWorkerClient {
    async fn history(
        &self,
        project_id: Option<i64>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<TaskInfoSummary>, AppError> {
        self.command(
            Self::service_principal(),
            project_id,
            TASK_READ_HISTORY_OPERATION,
            GetTaskHistoryInput {
                project_id,
                offset,
                limit,
            },
        )
        .await
    }
}

#[async_trait::async_trait]
impl WorkflowTaskLookupApplicationPort for WorkflowReadWorkerClient {
    async fn task_by_id(&self, task_id: &str) -> Result<Option<TaskLookupRecord>, AppError> {
        self.service_command(
            TASK_LOOKUP_BY_ID_OPERATION,
            TaskIdInput {
                task_id: task_id.to_string(),
            },
        )
        .await
    }

    async fn active_tasks(&self) -> Result<Vec<ActiveTaskLookupRecord>, AppError> {
        self.service_command(TASK_LOOKUP_ACTIVE_OPERATION, EmptyInput {})
            .await
    }

    async fn workflow_access_by_active_task_id(
        &self,
        task_id: &str,
    ) -> Result<Option<WorkflowTaskAccessRecord>, AppError> {
        self.service_command(
            WORKFLOW_ACCESS_ACTIVE_TASK_OPERATION,
            TaskIdInput {
                task_id: task_id.to_string(),
            },
        )
        .await
    }

    async fn workflow_run_for_resume(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowResumeRecord, AppError> {
        self.service_command(
            WORKFLOW_READ_RESUME_RECORD_OPERATION,
            WorkflowRunIdInput { workflow_run_id },
        )
        .await
    }

    async fn workflow_run_summary(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowRunSummary, AppError> {
        self.service_command(
            WORKFLOW_READ_SUMMARY_OPERATION,
            WorkflowRunIdInput { workflow_run_id },
        )
        .await
    }

    async fn workflow_progress_events(
        &self,
        workflow_run_id: i64,
        limit: usize,
        after_event_id: Option<i64>,
    ) -> Result<Vec<WorkflowProgressEvent>, AppError> {
        self.service_command(
            WORKFLOW_READ_PROGRESS_EVENTS_OPERATION,
            WorkflowProgressEventsInput {
                task_id: None,
                workflow_run_id: Some(workflow_run_id),
                after_event_id,
                limit,
            },
        )
        .await
    }
}
