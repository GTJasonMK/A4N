//! 大纲 DB 错误映射（单一来源）。

use a4n_core::error::AppError;
use a4n_db::outline::OutlineDbError;

pub(crate) fn outline_db_error(error: OutlineDbError) -> AppError {
    match error {
        OutlineDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        OutlineDbError::Database(error) => AppError::Database(error),
        OutlineDbError::Generation(error) => crate::generation::generation_error(error),
    }
}
