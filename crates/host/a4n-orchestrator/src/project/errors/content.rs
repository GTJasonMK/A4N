use super::lifecycle::project_status_write_error_to_app_error;
use a4n_db::project::{
    BlueprintRestoreError, BlueprintSyncError, ForeshadowingDbError, ProjectChapterContextError,
    ProjectChapterSceneRecordError, ProjectChapterSceneWriteError, ProjectOutlineRecordError,
};
use a4n_error::AppError;

pub(crate) fn blueprint_restore_error_to_app(error: BlueprintRestoreError) -> AppError {
    match error {
        BlueprintRestoreError::InvalidArgument(message)
        | BlueprintRestoreError::InvalidSnapshot(message) => AppError::InvalidArgument(message),
        BlueprintRestoreError::VersionNotFound { .. }
        | BlueprintRestoreError::ProjectNotFound { .. } => AppError::NotFound(error.to_string()),
        BlueprintRestoreError::InvalidProjectStatus { status, .. } => AppError::InvalidState {
            current: status,
            operation: "恢复蓝图版本".to_string(),
        },
        BlueprintRestoreError::CharacterSync { source }
        | BlueprintRestoreError::RelationshipSync { source } => match source {
            BlueprintSyncError::InvalidArgument(message) => AppError::InvalidArgument(message),
            BlueprintSyncError::Database(source) => AppError::database(source),
        },
        BlueprintRestoreError::StatusWrite { source } => {
            project_status_write_error_to_app_error(source)
        }
        BlueprintRestoreError::Serialization { source } => AppError::Serialization(source),
        BlueprintRestoreError::Database { source, .. } => AppError::database(source),
    }
}

pub(crate) fn project_outline_record_error_to_app_error(
    error: ProjectOutlineRecordError,
) -> AppError {
    match error {
        ProjectOutlineRecordError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ProjectOutlineRecordError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        ProjectOutlineRecordError::ChapterSourceChanged { .. } => AppError::InvalidState {
            current: "chapter_source_changed".to_string(),
            operation: error.to_string(),
        },
        ProjectOutlineRecordError::OutlineSourceChanged { .. } => AppError::InvalidState {
            current: "outline_source_changed".to_string(),
            operation: error.to_string(),
        },
        ProjectOutlineRecordError::Generation(error) => crate::generation::generation_error(error),
        ProjectOutlineRecordError::Database(error) => AppError::database(error),
    }
}

pub(crate) fn project_chapter_scene_write_error_to_app_error(
    error: ProjectChapterSceneWriteError,
) -> AppError {
    match error {
        ProjectChapterSceneWriteError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        ProjectChapterSceneWriteError::NotFound(message) => AppError::NotFound(message),
        ProjectChapterSceneWriteError::Database(error) => AppError::database(error),
    }
}

pub(crate) fn project_chapter_scene_record_error_to_app_error(
    error: ProjectChapterSceneRecordError,
) -> AppError {
    match error {
        ProjectChapterSceneRecordError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        ProjectChapterSceneRecordError::Database { source, .. } => AppError::database(source),
    }
}

pub(crate) fn project_chapter_context_error_to_app_error(
    error: ProjectChapterContextError,
) -> AppError {
    match error {
        ProjectChapterContextError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ProjectChapterContextError::NotFound(message) => AppError::NotFound(message),
        ProjectChapterContextError::Database(error) => AppError::database(error),
    }
}

pub(crate) fn foreshadowing_db_error_to_app_error(error: ForeshadowingDbError) -> AppError {
    match error {
        error @ (ForeshadowingDbError::ProjectNotFound { .. }
        | ForeshadowingDbError::RecordNotFound { .. }) => AppError::NotFound(error.to_string()),
        ForeshadowingDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ForeshadowingDbError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        error @ (ForeshadowingDbError::InvalidResolutionTarget { .. }
        | ForeshadowingDbError::DuplicateResolutionTarget { .. }) => {
            AppError::InvalidArgument(error.to_string())
        }
        ForeshadowingDbError::Database(error) => AppError::database(error),
    }
}
