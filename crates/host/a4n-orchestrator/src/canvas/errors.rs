use a4n_error::AppError;

use a4n_db::canvas::VideoStudioProjectImportError;

pub(crate) fn canvas_db_error(error: a4n_db::canvas::CanvasDbError) -> AppError {
    match error {
        a4n_db::canvas::CanvasDbError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::canvas::CanvasDbError::NotFound(message) => AppError::NotFound(message),
        a4n_db::canvas::CanvasDbError::Forbidden(message) => AppError::Forbidden(message),
        a4n_db::canvas::CanvasDbError::VersionConflict {
            graph_id,
            expected_version,
            current_version,
        } => AppError::InvalidState {
            current: format!(
                "canvas_graph_version_conflict:graph_id={graph_id},expected={expected_version},current={current_version}"
            ),
            operation: "保存画布".to_string(),
        },
        a4n_db::canvas::CanvasDbError::TextAssetVersionConflict {
            canvas_text_asset_id,
            expected_version,
            current_version,
        } => AppError::InvalidState {
            current: format!(
                "canvas_text_asset_version_conflict:asset_id={canvas_text_asset_id},expected={expected_version},current={current_version}"
            ),
            operation: "保存文本素材".to_string(),
        },
        a4n_db::canvas::CanvasDbError::Database(error) => AppError::database(error),
    }
}

pub(crate) fn video_studio_project_import_error(error: VideoStudioProjectImportError) -> AppError {
    match error {
        VideoStudioProjectImportError::InvalidPackage(message) => {
            AppError::InvalidArgument(message)
        }
        VideoStudioProjectImportError::MediaWrite(error) => {
            AppError::Internal(anyhow::anyhow!(error))
        }
        VideoStudioProjectImportError::ProjectInsert(error)
        | VideoStudioProjectImportError::TransactionCommit(error)
        | VideoStudioProjectImportError::Database(error) => AppError::database(error),
    }
}
