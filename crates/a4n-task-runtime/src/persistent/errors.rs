use a4n_error::AppError;

pub(in crate::persistent) fn queue_db_error(err: a4n_db::queue::QueueDbError) -> AppError {
    match err {
        a4n_db::queue::QueueDbError::Cancelled => AppError::Cancelled,
        a4n_db::queue::QueueDbError::InvalidConfig(message) => AppError::ConfigError(message),
        a4n_db::queue::QueueDbError::NotFound(message) => AppError::NotFound(message),
        a4n_db::queue::QueueDbError::AdmissionBlocked {
            package_id,
            reason_code,
        } => AppError::InvalidState {
            current: format!("package_id={package_id} reason_code={reason_code}"),
            operation: "创建持久化任务".to_string(),
        },
        a4n_db::queue::QueueDbError::Database(err) => AppError::Database(err),
    }
}
