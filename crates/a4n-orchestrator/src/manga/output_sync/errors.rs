use a4n_core::error::AppError;
use a4n_db::manga::MangaPipelineDbError;

pub fn manga_pipeline_db_error(error: MangaPipelineDbError) -> AppError {
    match error {
        MangaPipelineDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        MangaPipelineDbError::Database(err) => AppError::Database(err),
        MangaPipelineDbError::Generation(err) => crate::generation::generation_error(err),
    }
}
