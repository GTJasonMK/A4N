use a4n_api_contract::workflow::{WorkflowCancellationResult, WorkflowLifecycleApplicationPort};
use a4n_error::AppError;
use a4n_workflow_lifecycle_worker_contract::{
    OperationAck, ResumeWorkflowInput, WORKFLOW_LIFECYCLE_CANCEL_OPERATION,
    WORKFLOW_LIFECYCLE_MARK_RUNNING_OPERATION, WORKFLOW_LIFECYCLE_RESUME_OPERATION,
    WorkflowCancellationInput, WorkflowRunIdInput,
};

use super::WorkflowLifecycleWorkerClient;

#[async_trait::async_trait]
impl WorkflowLifecycleApplicationPort for WorkflowLifecycleWorkerClient {
    async fn mark_running(&self, workflow_run_id: i64) -> Result<(), AppError> {
        self.command::<_, OperationAck>(
            WORKFLOW_LIFECYCLE_MARK_RUNNING_OPERATION,
            WorkflowRunIdInput { workflow_run_id },
        )
        .await
        .map(|_| ())
    }

    async fn cancel(
        &self,
        workflow_run_id: i64,
        reason: &str,
    ) -> Result<WorkflowCancellationResult, AppError> {
        self.command(
            WORKFLOW_LIFECYCLE_CANCEL_OPERATION,
            WorkflowCancellationInput {
                workflow_run_id,
                reason: reason.to_string(),
            },
        )
        .await
    }

    async fn resume_with_active_task(
        &self,
        workflow_run_id: i64,
        active_task_id: &str,
    ) -> Result<(), AppError> {
        self.command::<_, OperationAck>(
            WORKFLOW_LIFECYCLE_RESUME_OPERATION,
            ResumeWorkflowInput {
                workflow_run_id,
                active_task_id: active_task_id.to_string(),
            },
        )
        .await
        .map(|_| ())
    }
}
