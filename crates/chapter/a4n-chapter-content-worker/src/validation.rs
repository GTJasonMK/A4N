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
