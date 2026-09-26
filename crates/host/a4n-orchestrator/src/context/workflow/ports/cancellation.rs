//! workflow cancellation port。

use super::*;

#[async_trait::async_trait]
impl WorkflowCancellationPort for OrchestratorContext {
    async fn mark_running(&self, workflow_run_id: i64) -> Result<(), AppError> {
        self.mark_workflow_running_validated(workflow_run_id)
            .await
            .map(|_| ())
    }

    async fn cancel(&self, workflow_run_id: i64, reason: &str) -> Result<(), AppError> {
        self.cancel_workflow_run_and_interrupt_videos(workflow_run_id, reason)
            .await
            .map(|_| ())
    }

    async fn resume_with_active_task(
        &self,
        workflow_run_id: i64,
        active_task_id: &str,
    ) -> Result<(), AppError> {
        self.resume_workflow_run_with_active_task_validated(workflow_run_id, active_task_id)
            .await
            .map(|_| ())
    }
}

#[async_trait::async_trait]
impl WorkflowCancellationSideEffectsPort for OrchestratorContext {
    async fn apply_cancellation_side_effects(
        &self,
        cancellation: WorkflowCancellationResult,
        reason: &str,
    ) -> Result<(), AppError> {
        self.apply_workflow_cancellation_side_effects(cancellation, reason)
            .await
    }
}
