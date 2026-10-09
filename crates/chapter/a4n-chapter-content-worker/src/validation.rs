use super::{CHAPTER_CONTENT_WORKER_ID, errors::WorkerFailure};
use a4n_worker_contract::WorkerEnvelope;

pub(super) fn validate_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    request.validate_worker_command(
        operation,
        CHAPTER_CONTENT_WORKER_ID,
        "chapter content worker",
    )
}

pub(super) fn user_id<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
) -> Result<i64, WorkerFailure> {
    request.principal.require_user_id()
}

pub(super) fn positive(field: &str, value: i64) -> Result<i64, WorkerFailure> {
    if value <= 0 {
        return Err(WorkerFailure::invalid_request(format!(
            "{field} 必须大于 0"
        )));
    }
    Ok(value)
}
