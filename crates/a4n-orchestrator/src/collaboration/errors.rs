use a4n_core::error::AppError;

use super::CollaborationDbError;

pub(super) fn collaboration_db_error_to_app_error(error: CollaborationDbError) -> AppError {
    match error {
        CollaborationDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        CollaborationDbError::NotFound(message) => AppError::NotFound(message),
        CollaborationDbError::Forbidden(message) => AppError::Forbidden(message),
        CollaborationDbError::InvalidState(message) => AppError::InvalidState {
            current: message,
            operation: "执行项目协作操作".to_string(),
        },
        CollaborationDbError::Internal(message) => AppError::Internal(anyhow::anyhow!(message)),
        CollaborationDbError::Database(error) => AppError::Database(error),
    }
}
