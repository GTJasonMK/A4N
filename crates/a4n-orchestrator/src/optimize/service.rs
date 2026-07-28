use a4n_core::error::AppError;
use a4n_db::optimize::OptimizeDbError;

pub fn optimize_db_error(error: OptimizeDbError) -> AppError {
    match error {
        OptimizeDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        OptimizeDbError::Database(error) => AppError::Database(error),
    }
}
