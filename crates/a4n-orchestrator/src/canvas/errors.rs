use a4n_core::error::AppError;

use super::VideoStudioProjectImportError;

pub(super) fn canvas_db_error(error: a4n_db::canvas::CanvasDbError) -> AppError {
    match error {
        a4n_db::canvas::CanvasDbError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::canvas::CanvasDbError::NotFound(message) => AppError::NotFound(message),
        a4n_db::canvas::CanvasDbError::Forbidden(message) => AppError::Forbidden(message),
        a4n_db::canvas::CanvasDbError::Database(error) => AppError::Database(error),
    }
}

pub(super) fn video_studio_project_import_error(error: VideoStudioProjectImportError) -> AppError {
    match error {
        VideoStudioProjectImportError::InvalidPackage(message) => {
            AppError::InvalidArgument(message)
        }
        VideoStudioProjectImportError::MediaWrite(error) => {
            AppError::Internal(anyhow::anyhow!(error))
        }
        VideoStudioProjectImportError::ProjectInsert(error)
        | VideoStudioProjectImportError::TransactionCommit(error)
        | VideoStudioProjectImportError::Database(error) => AppError::Database(error),
    }
}
