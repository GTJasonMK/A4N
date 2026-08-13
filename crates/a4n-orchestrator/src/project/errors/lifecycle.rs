use a4n_core::error::AppError;
use a4n_db::project::{
    ProjectDuplicateError, ProjectMutationError, ProjectStatusReadError, ProjectStatusWriteError,
};
use a4n_db::project_reset::ProjectResetDbError;

pub(crate) fn project_mutation_error_to_app_error(error: ProjectMutationError) -> AppError {
    match error {
        ProjectMutationError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ProjectMutationError::NotFound(message) => AppError::NotFound(message),
        ProjectMutationError::Forbidden(message) => AppError::Forbidden(message),
        ProjectMutationError::Internal(message) => AppError::Internal(anyhow::anyhow!(message)),
        ProjectMutationError::Database(error) => AppError::Database(error),
    }
}

pub(crate) fn project_status_read_error_to_app_error(error: ProjectStatusReadError) -> AppError {
    match error {
        ProjectStatusReadError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ProjectStatusReadError::Database { source, .. } => AppError::Database(source),
    }
}

pub(crate) fn project_status_write_error_to_app_error(error: ProjectStatusWriteError) -> AppError {
    match error {
        ProjectStatusWriteError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ProjectStatusWriteError::ProjectNotFound { project_id } => {
            AppError::NotFound(format!("项目不存在: {project_id}"))
        }
        ProjectStatusWriteError::StatusChanged { expected, actual } => AppError::InvalidState {
            current: actual,
            operation: format!("更新项目状态（期望当前状态 {expected}）"),
        },
        ProjectStatusWriteError::Database { source } => AppError::Database(source),
    }
}

pub(crate) fn project_reset_error_to_app_error(error: ProjectResetDbError) -> AppError {
    match error {
        ProjectResetDbError::ProjectNotFound { project_id } => {
            AppError::NotFound(format!("项目不存在: {project_id}"))
        }
        ProjectResetDbError::UnsupportedProjectKind { project_kind } => {
            AppError::InvalidArgument(format!("重置生成态不支持项目类型: {project_kind}"))
        }
        ProjectResetDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ProjectResetDbError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        ProjectResetDbError::StatusChanged { expected, actual } => AppError::InvalidState {
            current: actual,
            operation: format!("更新项目状态（期望当前状态 {expected}）"),
        },
        ProjectResetDbError::Internal(message) => AppError::Internal(anyhow::anyhow!(message)),
        ProjectResetDbError::CharacterProfileDelete { source } => {
            crate::character::character_profile_delete_error(source)
        }
        ProjectResetDbError::Generation { source } => crate::generation::generation_error(source),
        ProjectResetDbError::ProjectCleanup { source } => {
            crate::errors::project_delete_error_to_app_error(source)
        }
        ProjectResetDbError::ProjectRecordCleanup { source } => {
            crate::errors::project_cleanup_error_to_app_error(source)
        }
        ProjectResetDbError::ProjectStatusWrite { source } => {
            project_status_write_error_to_app_error(source)
        }
        ProjectResetDbError::ProjectMutation { source } => {
            project_mutation_error_to_app_error(source)
        }
        ProjectResetDbError::Database { source } => AppError::Database(source),
    }
}

pub(crate) fn project_duplicate_error_to_app_error(error: ProjectDuplicateError) -> AppError {
    match error {
        ProjectDuplicateError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ProjectDuplicateError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        ProjectDuplicateError::MediaCopy(error) => {
            AppError::Internal(anyhow::anyhow!("媒体文件复制失败：{error}"))
        }
        ProjectDuplicateError::Database(error) => AppError::Database(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_change_conflict_maps_to_invalid_state() {
        let error =
            project_status_write_error_to_app_error(ProjectStatusWriteError::StatusChanged {
                expected: "draft".to_string(),
                actual: "writing".to_string(),
            });

        assert!(matches!(
            error,
            AppError::InvalidState { current, operation }
                if current == "writing" && operation.contains("draft")
        ));
    }

    #[test]
    fn unsupported_reset_kind_maps_to_invalid_argument() {
        let error = project_reset_error_to_app_error(ProjectResetDbError::UnsupportedProjectKind {
            project_kind: "unknown".to_string(),
        });

        assert!(matches!(
            error,
            AppError::InvalidArgument(message) if message.contains("unknown")
        ));
    }
}
