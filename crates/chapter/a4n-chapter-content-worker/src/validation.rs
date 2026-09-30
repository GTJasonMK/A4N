use super::{CHAPTER_CONTENT_WORKER_ID, errors::WorkerFailure};
use a4n_worker_contract::{WorkerEnvelope, WorkerPrincipalKind};

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
    if request.lease.worker_id != CHAPTER_CONTENT_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 chapter content worker",
        ));
    }
    Ok(())
}

pub(super) fn user_id<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
) -> Result<i64, WorkerFailure> {
    if request.principal.kind != WorkerPrincipalKind::User {
        return Err(WorkerFailure::unauthorized("仅允许用户 principal"));
    }
    request
        .principal
        .subject
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| WorkerFailure::unauthorized("用户 principal subject 无效"))
}

pub(super) fn positive(field: &str, value: i64) -> Result<i64, WorkerFailure> {
    if value <= 0 {
        return Err(WorkerFailure::invalid_request(format!(
            "{field} 必须大于 0"
        )));
    }
    Ok(value)
}
