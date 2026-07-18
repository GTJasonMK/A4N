use a4n_core::error::AppError;

pub(in crate::task_queue) fn queue_db_error(err: a4n_db::queue::QueueDbError) -> AppError {
    match err {
        a4n_db::queue::QueueDbError::InvalidConfig(message) => AppError::ConfigError(message),
        a4n_db::queue::QueueDbError::NotFound(message) => AppError::NotFound(message),
        a4n_db::queue::QueueDbError::Database(err) => AppError::Database(err),
    }
}
