use a4n_db::optimize::OptimizeDbError;
use a4n_error::AppError;

pub fn optimize_db_error(error: OptimizeDbError) -> AppError {
    match error {
        OptimizeDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        OptimizeDbError::NotFound(message) => AppError::NotFound(message),
        OptimizeDbError::Forbidden(message) => AppError::Forbidden(message),
        OptimizeDbError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        OptimizeDbError::Database(error) => AppError::database(error),
    }
}
