use a4n_db::{project::ResolveChapterSourceError, shooting_script::ShootingScriptDbError};
use a4n_error::AppError;

pub(crate) fn resolve_source_error(error: ResolveChapterSourceError) -> AppError {
    match error {
        ResolveChapterSourceError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ResolveChapterSourceError::InvalidState(message) => AppError::InvalidState {
            current: message,
            operation: "解析章节拍摄剧本来源".to_string(),
        },
        ResolveChapterSourceError::NotFound(message) => AppError::NotFound(message),
        ResolveChapterSourceError::Database(error) => AppError::database(error),
    }
}

pub(crate) fn shooting_script_db_error(error: ShootingScriptDbError) -> AppError {
    match error {
        ShootingScriptDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ShootingScriptDbError::InvalidState(message) => AppError::InvalidState {
            current: message,
            operation: "读写章节共享拍摄剧本".to_string(),
        },
        ShootingScriptDbError::NotFound(message) => AppError::NotFound(message),
        ShootingScriptDbError::RevisionConflict {
            record_id,
            expected_revision,
        } => AppError::InvalidState {
            current: format!(
                "shooting_script_revision_conflict:id={record_id},expected={expected_revision}"
            ),
            operation: "更新章节共享拍摄剧本".to_string(),
        },
        ShootingScriptDbError::Database(error) => AppError::database(error),
    }
}
