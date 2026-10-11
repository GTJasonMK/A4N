use a4n_manga_record_worker_contract::MANGA_RECORD_WORKER_ID;
use a4n_worker_contract::WorkerEnvelope;
pub(super) use a4n_worker_contract::WorkerFailure;

pub(super) fn validate_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    request.validate_worker_command(operation, MANGA_RECORD_WORKER_ID, "manga record worker")
}

pub(super) fn manga_error(error: a4n_db::manga::MangaPromptRecordError) -> WorkerFailure {
    match error {
        a4n_db::manga::MangaPromptRecordError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::manga::MangaPromptRecordError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::manga::MangaPromptRecordError::Conflict(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::manga::MangaPromptRecordError::Database { source, .. } => {
            a4n_worker_db_adapter::map_database_error(source)
        }
        a4n_db::manga::MangaPromptRecordError::Generation(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}
