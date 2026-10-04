use a4n_api_contract::workflow::WorkflowCancellationResult;
use a4n_worker_contract::WorkerEnvelope;
use a4n_workflow_lifecycle_worker_contract::{
    OperationAck, ResumeWorkflowInput, WORKFLOW_LIFECYCLE_CANCEL_OPERATION,
    WORKFLOW_LIFECYCLE_MARK_RUNNING_OPERATION, WORKFLOW_LIFECYCLE_RESUME_OPERATION,
    WorkflowCancellationInput, WorkflowRunIdInput,
};

use super::{
    WorkerState,
    access::require_service_principal,
    protocol::{WorkerFailure, validate_request, workflow_error},
};

impl WorkerState {
    pub(super) async fn mark_running(
        &self,
        request: WorkerEnvelope<WorkflowRunIdInput>,
    ) -> Result<OperationAck, WorkerFailure> {
        validate_request(&request, WORKFLOW_LIFECYCLE_MARK_RUNNING_OPERATION)?;
        require_service_principal(&request.principal)?;
        a4n_db::workflow::resume_workflow_run(
            self.database.executor(),
            request.payload.workflow_run_id,
        )
        .await
        .map_err(workflow_error)?;
        Ok(OperationAck { accepted: true })
    }

    pub(super) async fn cancel(
        &self,
        request: WorkerEnvelope<WorkflowCancellationInput>,
    ) -> Result<WorkflowCancellationResult, WorkerFailure> {
        validate_request(&request, WORKFLOW_LIFECYCLE_CANCEL_OPERATION)?;
        require_service_principal(&request.principal)?;
        if request.payload.reason.trim().is_empty() {
            return Err(WorkerFailure::invalid_request("取消原因不能为空"));
        }
        let result = a4n_db::workflow::cancel_workflow_run_persistence(
            self.database.executor(),
            request.payload.workflow_run_id,
            &request.payload.reason,
        )
        .await
        .map_err(workflow_error)?;
        Ok(WorkflowCancellationResult {
            workflow_run_id: result.workflow_run_id,
            active_task_id: result.active_task_id,
            interrupted_generated_video_ids: result.interrupted_generated_video_ids,
            already_terminal: result.already_terminal,
            cancelled: result.cancelled,
        })
    }

    pub(super) async fn resume(
        &self,
        request: WorkerEnvelope<ResumeWorkflowInput>,
    ) -> Result<OperationAck, WorkerFailure> {
        validate_request(&request, WORKFLOW_LIFECYCLE_RESUME_OPERATION)?;
        require_service_principal(&request.principal)?;
        if request.payload.active_task_id.trim().is_empty() {
            return Err(WorkerFailure::invalid_request("active_task_id 不能为空"));
        }
        a4n_db::workflow::resume_workflow_run_with_active_task(
            self.database.executor(),
            request.payload.workflow_run_id,
            &request.payload.active_task_id,
        )
        .await
        .map_err(workflow_error)?;
        Ok(OperationAck { accepted: true })
    }
}
