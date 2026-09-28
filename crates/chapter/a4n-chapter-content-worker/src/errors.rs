use a4n_worker_contract::WorkerErrorCode;

#[derive(Debug)]
pub(super) struct WorkerFailure {
    pub(super) code: WorkerErrorCode,
    pub(super) message: String,
    pub(super) retryable: bool,
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
            WorkerFailure::unavailable(error.to_string())
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

pub(super) fn workflow_error(error: a4n_db::workflow::WorkflowReadError) -> WorkerFailure {
    match error {
        a4n_db::workflow::WorkflowReadError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::workflow::WorkflowReadError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_db::workflow::WorkflowReadError::InvalidData(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::workflow::WorkflowReadError::Database(error) => {
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
