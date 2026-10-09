use a4n_manga_record_worker_contract::MANGA_RECORD_WORKER_ID;
use a4n_worker_contract::WorkerEnvelope;
pub(super) use a4n_worker_contract::WorkerFailure;

pub(super) fn validate_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    request.validate_worker_command(operation, MANGA_RECORD_WORKER_ID, "manga record worker")
}

pub(super) fn positive(value: i64, field: &str) -> Result<i64, WorkerFailure> {
    if value > 0 {
        Ok(value)
    } else {
        Err(WorkerFailure::invalid_request(format!(
            "{field} 必须大于 0"
        )))
    }
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
            WorkerFailure::unavailable(source.to_string())
        }
        a4n_db::manga::MangaPromptRecordError::Generation(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

pub(super) fn workflow_error(error: a4n_db::workflow::WorkflowReadError) -> WorkerFailure {
    match error {
        a4n_db::workflow::WorkflowReadError::InvalidArgument(message)
        | a4n_db::workflow::WorkflowReadError::InvalidData(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::workflow::WorkflowReadError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_db::workflow::WorkflowReadError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

pub(super) fn project_family_error(error: a4n_db::project::ProjectFamilyDbError) -> WorkerFailure {
    match error {
        a4n_db::project::ProjectFamilyDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ProjectFamilyDbError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::project::ProjectFamilyDbError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

pub(super) fn collaboration_error(
    error: a4n_db::collaboration::CollaborationDbError,
) -> WorkerFailure {
    match error {
        a4n_db::collaboration::CollaborationDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::collaboration::CollaborationDbError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::collaboration::CollaborationDbError::Forbidden(message) => {
            WorkerFailure::unauthorized(message)
        }
        a4n_db::collaboration::CollaborationDbError::InvalidState(message)
        | a4n_db::collaboration::CollaborationDbError::Internal(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::collaboration::CollaborationDbError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}
