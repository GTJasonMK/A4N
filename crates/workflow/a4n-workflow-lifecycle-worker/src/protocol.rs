pub(super) use a4n_worker_contract::WorkerFailure;
use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};
use a4n_workflow_lifecycle_worker_contract::WORKFLOW_LIFECYCLE_WORKER_ID;

pub(super) fn validate_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    request
        .validate()
        .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
    if request.operation_id != operation {
        return Err(WorkerFailure::invalid_request(format!(
            "operation_id 不匹配: expected={operation}, actual={}",
            request.operation_id
        )));
    }
    if request.lease.worker_id != WORKFLOW_LIFECYCLE_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 workflow lifecycle worker",
        ));
    }
    Ok(())
}

pub(super) fn workflow_error(error: a4n_db::workflow::WorkflowWriteError) -> WorkerFailure {
    match error {
        a4n_db::workflow::WorkflowWriteError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::workflow::WorkflowWriteError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::workflow::WorkflowWriteError::StatusConflict { current, operation } => {
            WorkerFailure::conflict(format!("当前状态 {current} 不允许执行 {operation}"))
        }
        a4n_db::workflow::WorkflowWriteError::InvalidData(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::workflow::WorkflowWriteError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

pub(super) fn completed<T>(
    request: &WorkerEnvelope<impl serde::Serialize>,
    payload: T,
) -> WorkerResponse<T> {
    WorkerResponse::completed_for(request, payload)
}

pub(super) fn failed<T>(
    request: &WorkerEnvelope<impl serde::Serialize>,
    failure: WorkerFailure,
) -> WorkerResponse<T> {
    WorkerResponse::failed_for(request, failure.into())
}
