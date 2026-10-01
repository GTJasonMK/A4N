use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use super::WorkerClientError;

fn validate_response<T>(response: &WorkerResponse<T>) -> Result<(), WorkerClientError> {
    response
        .validate()
        .map_err(WorkerClientError::InvalidResponse)
}

pub(super) fn validate_command_response<I, O>(
    request: &WorkerEnvelope<I>,
    response: &WorkerResponse<O>,
) -> Result<(), WorkerClientError> {
    validate_response(response)?;
    if response.request_id != request.request_id {
        return Err(WorkerClientError::ResponseIdentityMismatch {
            field: "request_id",
        });
    }
    if response.trace_id != request.trace_id {
        return Err(WorkerClientError::ResponseIdentityMismatch { field: "trace_id" });
    }
    Ok(())
}

pub(super) fn validate_operation_response<T>(
    response: &WorkerResponse<T>,
    operation_id: &str,
) -> Result<(), WorkerClientError> {
    validate_response(response)?;
    if response.operation.operation_id != operation_id {
        return Err(WorkerClientError::ResponseIdentityMismatch {
            field: "operation.operation_id",
        });
    }
    Ok(())
}
