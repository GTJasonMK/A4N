use super::*;

#[async_trait::async_trait]
impl WorkflowReadApplicationPort for WorkflowReadWorkerClient {
    async fn list_for_user(
        &self,
        user_id: i64,
        input: ListWorkflowRunsInput,
    ) -> Result<Vec<WorkflowRunSummary>, AppError> {
        self.command(
            a4n_worker_client::user_principal(user_id)?,
            input.project_id,
            WORKFLOW_READ_LIST_OPERATION,
            input,
        )
        .await
    }

    async fn detail_for_user(
        &self,
        user_id: i64,
        input: GetWorkflowRunInput,
    ) -> Result<WorkflowRunDetailSummary, AppError> {
        self.command(
            a4n_worker_client::user_principal(user_id)?,
            None,
            WORKFLOW_READ_DETAIL_OPERATION,
            input,
        )
        .await
    }
}
