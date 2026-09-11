use a4n_db::project::{
    ProjectBasicRecordError, ProjectFamilyDbError, ProjectFamilyExportError,
    ProjectFamilyMetadataReadError, ProjectProgressReadError, ProjectRollbackImpactReadError,
};
use a4n_db::project_family_import::ProjectFamilyImportError;
use a4n_error::AppError;

pub(crate) fn project_family_error_to_app_error(error: ProjectFamilyDbError) -> AppError {
    match error {
        ProjectFamilyDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ProjectFamilyDbError::NotFound(message) => AppError::NotFound(message),
        ProjectFamilyDbError::Database(error) => AppError::Database(error),
    }
}

pub(crate) fn project_family_export_error_to_app_error(
    error: ProjectFamilyExportError,
) -> AppError {
    match error {
        ProjectFamilyExportError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ProjectFamilyExportError::NotFound(message) => AppError::NotFound(message),
        ProjectFamilyExportError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        ProjectFamilyExportError::Database(error) => AppError::Database(error),
    }
}

pub(crate) fn project_basic_record_error_to_app_error(error: ProjectBasicRecordError) -> AppError {
    match error {
        ProjectBasicRecordError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ProjectBasicRecordError::ProjectNotFound { .. } => AppError::NotFound(error.to_string()),
        ProjectBasicRecordError::Database { source, .. } => AppError::Database(source),
    }
}

pub(crate) fn project_family_metadata_read_error_to_app_error(
    error: ProjectFamilyMetadataReadError,
) -> AppError {
    match error {
        ProjectFamilyMetadataReadError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        ProjectFamilyMetadataReadError::RootProjectMissing { .. }
        | ProjectFamilyMetadataReadError::SourceSnapshotMissing { .. } => {
            AppError::Internal(anyhow::anyhow!(error.to_string()))
        }
        ProjectFamilyMetadataReadError::Database { source, .. } => AppError::Database(source),
    }
}

pub(crate) fn project_progress_read_error_to_app_error(
    error: ProjectProgressReadError,
) -> AppError {
    match error {
        ProjectProgressReadError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ProjectProgressReadError::InvalidProjectKind { .. }
        | ProjectProgressReadError::InvalidProjectStatus { .. }
        | ProjectProgressReadError::InvalidProjectSource { .. }
        | ProjectProgressReadError::InvalidChapterProgress { .. }
        | ProjectProgressReadError::InvalidPersistedStatus { .. }
        | ProjectProgressReadError::InvalidPersistedReference { .. }
        | ProjectProgressReadError::SummaryMissing { .. } => {
            AppError::Internal(anyhow::anyhow!(error.to_string()))
        }
        ProjectProgressReadError::Database { source, .. } => AppError::Database(source),
    }
}

pub(crate) fn project_rollback_impact_read_error_to_app_error(
    error: ProjectRollbackImpactReadError,
) -> AppError {
    match error {
        ProjectRollbackImpactReadError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        ProjectRollbackImpactReadError::Database { source, .. } => AppError::Database(source),
    }
}

pub(crate) fn project_family_import_error_to_app_error(
    error: ProjectFamilyImportError,
) -> AppError {
    match error {
        ProjectFamilyImportError::Database(source)
        | ProjectFamilyImportError::Commit(source)
        | ProjectFamilyImportError::DatabaseContext { source, .. } => AppError::Database(source),
        ProjectFamilyImportError::InvalidPackage(message) => AppError::InvalidArgument(message),
        ProjectFamilyImportError::MediaImport(source) => {
            AppError::Internal(anyhow::anyhow!(source.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_export_invalid_state_preserves_state_and_operation() {
        let error =
            project_family_export_error_to_app_error(ProjectFamilyExportError::InvalidState {
                current: "detached".to_string(),
                operation: "导出项目族".to_string(),
            });

        assert!(matches!(
            error,
            AppError::InvalidState { current, operation }
                if current == "detached" && operation == "导出项目族"
        ));
    }
}
