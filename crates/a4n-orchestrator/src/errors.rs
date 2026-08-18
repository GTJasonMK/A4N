pub(crate) fn app_error_with_secondary(
    error: a4n_core::error::AppError,
    secondary: impl Into<String>,
) -> a4n_core::error::AppError {
    a4n_core::error::AppError::InvalidState {
        current: error.to_string(),
        operation: secondary.into(),
    }
}

pub(crate) fn project_delete_error_to_app_error(
    error: a4n_db::project_delete::ProjectDeleteDbError,
) -> a4n_core::error::AppError {
    use a4n_core::error::AppError;
    use a4n_db::project_delete::ProjectDeleteDbError;

    match error {
        ProjectDeleteDbError::ProjectNotFound { .. } => AppError::NotFound(error.to_string()),
        ProjectDeleteDbError::DerivedProjectsExist { .. } => {
            AppError::InvalidArgument(error.to_string())
        }
        ProjectDeleteDbError::ProjectCleanup { source } => {
            project_cleanup_error_to_app_error(source)
        }
        ProjectDeleteDbError::Database { source } => AppError::Database(source),
        ProjectDeleteDbError::Generation { source } => match source {
            a4n_db::generation::GenerationDbError::InvalidArgument(message) => {
                AppError::InvalidArgument(message)
            }
            a4n_db::generation::GenerationDbError::NotFound(message) => AppError::NotFound(message),
            a4n_db::generation::GenerationDbError::Database(source) => AppError::Database(source),
        },
        ProjectDeleteDbError::Video { source } => match source {
            a4n_db::video::VideoDbError::InvalidArgument(message) => {
                AppError::InvalidArgument(message)
            }
            a4n_db::video::VideoDbError::InvalidState { current, operation } => {
                AppError::InvalidState { current, operation }
            }
            a4n_db::video::VideoDbError::NotFound(message) => AppError::NotFound(message),
            a4n_db::video::VideoDbError::Forbidden(message) => AppError::Forbidden(message),
            a4n_db::video::VideoDbError::Database(source) => AppError::Database(source),
        },
    }
}

pub(crate) fn project_cleanup_error_to_app_error(
    error: a4n_db::project::ProjectCleanupDbError,
) -> a4n_core::error::AppError {
    use a4n_core::error::AppError;

    match error {
        a4n_db::project::ProjectCleanupDbError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::project::ProjectCleanupDbError::NotFound(message) => AppError::NotFound(message),
        a4n_db::project::ProjectCleanupDbError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        a4n_db::project::ProjectCleanupDbError::Database { source } => AppError::Database(source),
        a4n_db::project::ProjectCleanupDbError::Generation { source } => {
            crate::generation::generation_error(source)
        }
        a4n_db::project::ProjectCleanupDbError::Video { source } => match source {
            a4n_db::video::VideoDbError::InvalidArgument(message) => {
                AppError::InvalidArgument(message)
            }
            a4n_db::video::VideoDbError::InvalidState { current, operation } => {
                AppError::InvalidState { current, operation }
            }
            a4n_db::video::VideoDbError::NotFound(message) => AppError::NotFound(message),
            a4n_db::video::VideoDbError::Forbidden(message) => AppError::Forbidden(message),
            a4n_db::video::VideoDbError::Database(source) => AppError::Database(source),
        },
    }
}

pub(crate) fn chapter_content_edit_error_to_app_error(
    error: a4n_db::chapter_content::ChapterContentEditError,
) -> a4n_core::error::AppError {
    use a4n_core::error::AppError;
    use a4n_db::chapter_content::ChapterContentEditError;

    match error {
        ChapterContentEditError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ChapterContentEditError::NotFound(message) => AppError::NotFound(message),
        ChapterContentEditError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        ChapterContentEditError::Database(error) => AppError::Database(error),
    }
}

pub(crate) fn freeform_generated_reset_error_to_app_error(
    error: a4n_db::project::FreeformGeneratedResetDbError,
) -> a4n_core::error::AppError {
    use a4n_core::error::AppError;

    match error {
        a4n_db::project::FreeformGeneratedResetDbError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::project::FreeformGeneratedResetDbError::ProjectNotFound { .. } => {
            AppError::NotFound(error.to_string())
        }
        a4n_db::project::FreeformGeneratedResetDbError::UnsupportedProject => {
            AppError::InvalidArgument(error.to_string())
        }
        a4n_db::project::FreeformGeneratedResetDbError::ProjectCleanup { source } => {
            project_cleanup_error_to_app_error(source)
        }
        a4n_db::project::FreeformGeneratedResetDbError::Canvas { source } => match source {
            a4n_db::canvas::CanvasDbError::InvalidArgument(message) => {
                AppError::InvalidArgument(message)
            }
            a4n_db::canvas::CanvasDbError::NotFound(message) => AppError::NotFound(message),
            a4n_db::canvas::CanvasDbError::Forbidden(message) => AppError::Forbidden(message),
            a4n_db::canvas::CanvasDbError::Database(source) => AppError::Database(source),
        },
        a4n_db::project::FreeformGeneratedResetDbError::Database { source } => {
            AppError::Database(source)
        }
    }
}
