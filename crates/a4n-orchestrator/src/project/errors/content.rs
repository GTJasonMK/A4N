use super::lifecycle::project_status_write_error_to_app_error;
use a4n_core::error::AppError;
use a4n_db::project::{
    BlueprintRestoreError, BlueprintSyncError, ForeshadowingDbError, ProjectChapterContextError,
    ProjectChapterOutlineUpdateError, ProjectChapterSceneRecordError,
    ProjectChapterSceneReorderError, ProjectGeneratedChapterSceneSaveError,
    ProjectOutlineRecordError,
};

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
            BlueprintSyncError::Database(source) => AppError::Database(source),
        },
        BlueprintRestoreError::StatusWrite { source } => {
            project_status_write_error_to_app_error(source)
        }
        BlueprintRestoreError::Serialization { source } => AppError::Serialization(source),
        BlueprintRestoreError::Database { source, .. } => AppError::Database(source),
    }
}

pub(crate) fn project_chapter_outline_update_error_to_app_error(
    error: ProjectChapterOutlineUpdateError,
) -> AppError {
    match error {
        ProjectChapterOutlineUpdateError::NotFound(message) => AppError::NotFound(message),
        ProjectChapterOutlineUpdateError::Forbidden(message) => AppError::Forbidden(message),
        ProjectChapterOutlineUpdateError::Database(error) => AppError::Database(error),
    }
}

pub(crate) fn project_outline_record_error_to_app_error(
    error: ProjectOutlineRecordError,
) -> AppError {
    match error {
        ProjectOutlineRecordError::Database(error) => AppError::Database(error),
    }
}

pub(crate) fn project_chapter_scene_reorder_error_to_app_error(
    error: ProjectChapterSceneReorderError,
) -> AppError {
    match error {
        ProjectChapterSceneReorderError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        ProjectChapterSceneReorderError::NotFound(message) => AppError::NotFound(message),
        ProjectChapterSceneReorderError::Database(error) => AppError::Database(error),
    }
}

pub(crate) fn project_generated_chapter_scene_save_error_to_app_error(
    error: ProjectGeneratedChapterSceneSaveError,
) -> AppError {
    match error {
        ProjectGeneratedChapterSceneSaveError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        ProjectGeneratedChapterSceneSaveError::NotFound(message) => AppError::NotFound(message),
        ProjectGeneratedChapterSceneSaveError::Database(error) => AppError::Database(error),
    }
}

pub(crate) fn project_chapter_scene_record_error_to_app_error(
    error: ProjectChapterSceneRecordError,
) -> AppError {
    match error {
        ProjectChapterSceneRecordError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        ProjectChapterSceneRecordError::Database { source, .. } => AppError::Database(source),
    }
}

pub(crate) fn project_chapter_context_error_to_app_error(
    error: ProjectChapterContextError,
) -> AppError {
    match error {
        ProjectChapterContextError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ProjectChapterContextError::NotFound(message) => AppError::NotFound(message),
        ProjectChapterContextError::Database(error) => AppError::Database(error),
    }
}

pub(crate) fn foreshadowing_db_error_to_app_error(error: ForeshadowingDbError) -> AppError {
    match error {
        ForeshadowingDbError::Database(error) => AppError::Database(error),
    }
}
