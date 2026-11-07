pub(super) use a4n_worker_contract::WorkerFailure;

pub(super) fn chapter_error(
    error: a4n_db::chapter_content::ChapterContentEditError,
) -> WorkerFailure {
    match error {
        a4n_db::chapter_content::ChapterContentEditError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::chapter_content::ChapterContentEditError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::chapter_content::ChapterContentEditError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}: {operation}"))
        }
        a4n_db::chapter_content::ChapterContentEditError::Database(error) => {
            a4n_worker_db_adapter::map_database_error(error)
        }
    }
}

pub(super) fn import_error(
    error: a4n_db::chapter_content::ChapterContentImportError,
) -> WorkerFailure {
    match error {
        a4n_db::chapter_content::ChapterContentImportError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::chapter_content::ChapterContentImportError::TargetChapterGenerating {
            chapter_number,
            status,
            operation,
        } => WorkerFailure::conflict(format!(
            "第 {chapter_number} 章正在生成中，请等待当前生成完成后再{operation}（当前状态：{status}）"
        )),
        a4n_db::chapter_content::ChapterContentImportError::Database(error) => {
            a4n_worker_db_adapter::map_database_error(error)
        }
    }
}

pub(super) fn generation_lifecycle_error(
    error: a4n_db::generation::GenerationLifecycleGateError,
) -> WorkerFailure {
    use a4n_db::generation::GenerationLifecycleGateError as Error;
    match error {
        Error::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        Error::NotFound(message) => WorkerFailure::not_found(message),
        Error::Blocked(message) => WorkerFailure::conflict(message),
        Error::Corrupt(message) => WorkerFailure::unavailable(message),
        Error::Database(error) => WorkerFailure::unavailable(error.to_string()),
    }
}

pub(super) fn project_error(error: a4n_db::project::ProjectMutationError) -> WorkerFailure {
    match error {
        a4n_db::project::ProjectMutationError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ProjectMutationError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::project::ProjectMutationError::Forbidden(message) => {
            WorkerFailure::unauthorized(message)
        }
        a4n_db::project::ProjectMutationError::Internal(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::project::ProjectMutationError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

pub(super) fn app_error(error: a4n_error::AppError) -> WorkerFailure {
    match error {
        a4n_error::AppError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        a4n_error::AppError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_error::AppError::Forbidden(message) => WorkerFailure::unauthorized(message),
        a4n_error::AppError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}: {operation}"))
        }
        other => WorkerFailure::unavailable(other.to_string()),
    }
}
