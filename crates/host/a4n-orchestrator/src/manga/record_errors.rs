use a4n_db::manga::{MangaPipelineDbError, MangaPromptRecordError};
use a4n_error::AppError;

pub(crate) fn manga_prompt_record_error(error: MangaPromptRecordError) -> AppError {
    match error {
        MangaPromptRecordError::InvalidArgument(message) => AppError::InvalidArgument(message),
        MangaPromptRecordError::NotFound(message) => AppError::NotFound(message),
        MangaPromptRecordError::Conflict(message) => AppError::InvalidState {
            current: message,
            operation: "创建漫画提示词".to_string(),
        },
        MangaPromptRecordError::Database { source, .. } => AppError::Database(source),
        MangaPromptRecordError::Generation(error) => crate::generation::generation_error(error),
    }
}

pub(in crate::manga) fn manga_pipeline_db_error(error: MangaPipelineDbError) -> AppError {
    match error {
        MangaPipelineDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        MangaPipelineDbError::Database(error) => AppError::Database(error),
        MangaPipelineDbError::Generation(error) => crate::generation::generation_error(error),
    }
}
