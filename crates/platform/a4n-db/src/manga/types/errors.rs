use crate::generation::GenerationDbError;
use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum MangaPromptRecordError {
    #[error("参数错误：{0}")]
    InvalidArgument(String),
    #[error("未找到：{0}")]
    NotFound(String),
    #[error("冲突：{0}")]
    Conflict(String),
    #[error("{operation}: {source}")]
    Database {
        operation: &'static str,
        #[source]
        source: DbErr,
    },
    #[error(transparent)]
    Generation(#[from] GenerationDbError),
}

pub(in crate::manga) fn manga_record_database(
    operation: &'static str,
) -> impl FnOnce(DbErr) -> MangaPromptRecordError {
    move |source| MangaPromptRecordError::Database { operation, source }
}

pub(in crate::manga) fn manga_video_error(
    error: crate::video::VideoDbError,
) -> MangaPromptRecordError {
    match error {
        crate::video::VideoDbError::Database(source) => MangaPromptRecordError::Database {
            operation: "删除漫画页成图失败",
            source,
        },
        crate::video::VideoDbError::NotFound(message) => MangaPromptRecordError::NotFound(message),
        error => MangaPromptRecordError::InvalidArgument(error.to_string()),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MangaPipelineDbError {
    #[error(transparent)]
    Database(#[from] DbErr),
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Generation(#[from] GenerationDbError),
}
