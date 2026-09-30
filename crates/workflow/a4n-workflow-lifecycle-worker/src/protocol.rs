use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerOperationRef,
    WorkerOperationStatus, WorkerResponse,
};
use a4n_workflow_lifecycle_worker_contract::WORKFLOW_LIFECYCLE_WORKER_ID;

pub(super) struct WorkerFailure {
    code: WorkerErrorCode,
    message: String,
    retryable: bool,
}

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
    WorkerResponse {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        trace_id: request.trace_id.clone(),
        operation: WorkerOperationRef {
            operation_id: request.operation_id.clone(),
            task_id: None,
        },
        status: WorkerOperationStatus::Completed,
        payload: Some(payload),
        resource_refs: Vec::new(),
        committed_revision: None,
        provider_job_id: None,
        result_summary: None,
        pagination: None,
        error: None,
    }
}

pub(super) fn failed<T>(
    request: &WorkerEnvelope<impl serde::Serialize>,
    failure: WorkerFailure,
) -> WorkerResponse<T> {
    WorkerResponse {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        trace_id: request.trace_id.clone(),
        operation: WorkerOperationRef {
            operation_id: request.operation_id.clone(),
            task_id: None,
        },
        status: WorkerOperationStatus::Failed,
        payload: None,
        resource_refs: Vec::new(),
        committed_revision: None,
        provider_job_id: None,
        result_summary: None,
        pagination: None,
        error: Some(WorkerError {
            code: failure.code,
            message: failure.message,
            retryable: failure.retryable,
        }),
    }
}

impl WorkerFailure {
    pub(super) fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::InvalidRequest,
            message: message.into(),
            retryable: false,
        }
    }

    pub(super) fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Unauthorized,
            message: message.into(),
            retryable: false,
        }
    }

    pub(super) fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::NotFound,
            message: message.into(),
            retryable: false,
        }
    }

    pub(super) fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Conflict,
            message: message.into(),
            retryable: false,
        }
    }

    pub(super) fn unavailable(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Unavailable,
            message: message.into(),
            retryable: true,
        }
    }
}
