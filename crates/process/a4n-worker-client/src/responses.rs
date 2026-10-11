use a4n_error::AppError;
use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use super::{WorkerClientError, WorkerError, WorkerOperationStatus};

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

pub fn worker_response_payload<T>(
    response: WorkerResponse<T>,
    worker_name: &str,
    operation: Option<&str>,
    map_error: impl FnOnce(&WorkerError) -> AppError,
) -> Result<T, AppError> {
    match response.status {
        WorkerOperationStatus::Completed => response.payload.ok_or_else(|| {
            AppError::ConfigError(match operation {
                Some(operation) => {
                    format!("{worker_name} 完成响应缺少 payload: {operation}")
                }
                None => format!("{worker_name} 完成响应缺少 payload"),
            })
        }),
        WorkerOperationStatus::Failed => Err(map_error(
            response.error.as_ref().expect("validated failed response"),
        )),
        status => Err(AppError::ConfigError(match operation {
            Some(operation) => {
                format!("{worker_name} 返回未完成状态: operation={operation}, status={status:?}")
            }
            None => format!("{worker_name} 返回未完成状态: {status:?}"),
        })),
    }
}
