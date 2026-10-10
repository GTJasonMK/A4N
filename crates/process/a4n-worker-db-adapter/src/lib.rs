use a4n_db::asset::AssetDbError;
use a4n_db::collaboration::CollaborationDbError;
use a4n_db::profile_material::ProfileMaterialDbError;
use a4n_db::project::{
    ProjectFamilyDbError, ProjectFamilyMetadataReadError, ProjectMutationError,
    ProjectProgressReadError,
};
use a4n_db::scene_profile::SceneProfileDbError;
use a4n_db::workflow::WorkflowReadError;
use a4n_error::AppError;
use a4n_worker_contract::WorkerFailure;

enum InternalErrorPolicy {
    Conflict,
    Internal,
}

enum InvalidDataPolicy {
    Conflict,
    InvalidRequest,
}

pub fn map_app_error(error: AppError) -> WorkerFailure {
    match error {
        AppError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        AppError::NotFound(message) => WorkerFailure::not_found(message),
        AppError::Forbidden(message) => WorkerFailure::unauthorized(message),
        AppError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}: {operation}"))
        }
        other => WorkerFailure::unavailable(other.to_string()),
    }
}

pub fn map_database_error(error: impl std::fmt::Display) -> WorkerFailure {
    WorkerFailure::unavailable(error.to_string())
}

pub fn map_asset_db_error(error: AssetDbError) -> WorkerFailure {
    match error {
        AssetDbError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        AssetDbError::NotFound(message) => WorkerFailure::not_found(message),
        AssetDbError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{operation}: {current}"))
        }
        AssetDbError::Database(error) => WorkerFailure::unavailable(error.to_string()),
    }
}

pub fn map_profile_material_db_error(error: ProfileMaterialDbError) -> WorkerFailure {
    match error {
        ProfileMaterialDbError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        ProfileMaterialDbError::Database { source, .. } => {
            WorkerFailure::unavailable(source.to_string())
        }
    }
}

pub fn map_scene_profile_db_error(error: SceneProfileDbError) -> WorkerFailure {
    match error {
        SceneProfileDbError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        SceneProfileDbError::NotFound(message) => WorkerFailure::not_found(message),
        SceneProfileDbError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{operation}: {current}"))
        }
        SceneProfileDbError::Database(error) => WorkerFailure::unavailable(error.to_string()),
    }
}

pub fn map_collaboration_db_error(error: CollaborationDbError) -> WorkerFailure {
    map_collaboration_error(error, InternalErrorPolicy::Conflict)
}

pub fn map_collaboration_db_error_preserving_internal(
    error: CollaborationDbError,
) -> WorkerFailure {
    map_collaboration_error(error, InternalErrorPolicy::Internal)
}

pub fn map_project_mutation_error(error: ProjectMutationError) -> WorkerFailure {
    match error {
        ProjectMutationError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        ProjectMutationError::NotFound(message) => WorkerFailure::not_found(message),
        ProjectMutationError::Forbidden(message) => WorkerFailure::unauthorized(message),
        ProjectMutationError::Internal(message) => WorkerFailure::conflict(message),
        ProjectMutationError::Database(error) => WorkerFailure::unavailable(error.to_string()),
    }
}

pub fn map_project_family_db_error(error: ProjectFamilyDbError) -> WorkerFailure {
    match error {
        ProjectFamilyDbError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        ProjectFamilyDbError::NotFound(message) => WorkerFailure::not_found(message),
        ProjectFamilyDbError::Database(error) => WorkerFailure::unavailable(error.to_string()),
    }
}

pub fn map_project_family_metadata_read_error(
    error: ProjectFamilyMetadataReadError,
) -> WorkerFailure {
    match error {
        ProjectFamilyMetadataReadError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        error @ (ProjectFamilyMetadataReadError::RootProjectMissing { .. }
        | ProjectFamilyMetadataReadError::SourceSnapshotMissing { .. }) => {
            WorkerFailure::conflict(error.to_string())
        }
        ProjectFamilyMetadataReadError::Database { source, .. } => {
            WorkerFailure::unavailable(source.to_string())
        }
    }
}

pub fn map_project_progress_read_error(error: ProjectProgressReadError) -> WorkerFailure {
    match error {
        ProjectProgressReadError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        error @ (ProjectProgressReadError::InvalidProjectKind { .. }
        | ProjectProgressReadError::InvalidProjectStatus { .. }
        | ProjectProgressReadError::InvalidProjectSource { .. }
        | ProjectProgressReadError::InvalidChapterProgress { .. }
        | ProjectProgressReadError::InvalidPersistedStatus { .. }
        | ProjectProgressReadError::InvalidPersistedReference { .. }
        | ProjectProgressReadError::SummaryMissing { .. }) => {
            WorkerFailure::conflict(error.to_string())
        }
        ProjectProgressReadError::Database { source, .. } => {
            WorkerFailure::unavailable(source.to_string())
        }
    }
}

pub fn map_workflow_read_error_as_conflict(error: WorkflowReadError) -> WorkerFailure {
    map_workflow_read_error(error, InvalidDataPolicy::Conflict)
}

pub fn map_workflow_read_error_as_invalid_request(error: WorkflowReadError) -> WorkerFailure {
    map_workflow_read_error(error, InvalidDataPolicy::InvalidRequest)
}

fn map_workflow_read_error(
    error: WorkflowReadError,
    invalid_data_policy: InvalidDataPolicy,
) -> WorkerFailure {
    match error {
        WorkflowReadError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        WorkflowReadError::NotFound(message) => WorkerFailure::not_found(message),
        WorkflowReadError::InvalidData(message) => match invalid_data_policy {
            InvalidDataPolicy::Conflict => WorkerFailure::conflict(message),
            InvalidDataPolicy::InvalidRequest => WorkerFailure::invalid_request(message),
        },
        WorkflowReadError::Database(error) => WorkerFailure::unavailable(error.to_string()),
    }
}

fn map_collaboration_error(
    error: CollaborationDbError,
    internal_error_policy: InternalErrorPolicy,
) -> WorkerFailure {
    match error {
        CollaborationDbError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        CollaborationDbError::NotFound(message) => WorkerFailure::not_found(message),
        CollaborationDbError::Forbidden(message) => WorkerFailure::unauthorized(message),
        CollaborationDbError::InvalidState(message) => WorkerFailure::conflict(message),
        CollaborationDbError::Internal(message) => match internal_error_policy {
            InternalErrorPolicy::Conflict => WorkerFailure::conflict(message),
            InternalErrorPolicy::Internal => WorkerFailure::internal(message),
        },
        CollaborationDbError::Database(error) => WorkerFailure::unavailable(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use a4n_worker_contract::{WorkerError, WorkerErrorCode};
    use sea_orm::DbErr;

    use super::*;

    #[test]
    fn collaboration_errors_map_to_the_worker_failure_contract() {
        let database_error = DbErr::Custom("database unavailable".into());
        let database_message = database_error.to_string();
        let cases = [
            (
                CollaborationDbError::InvalidArgument("invalid input".into()),
                WorkerErrorCode::InvalidRequest,
                false,
                "invalid input".to_string(),
            ),
            (
                CollaborationDbError::NotFound("missing member".into()),
                WorkerErrorCode::NotFound,
                false,
                "missing member".to_string(),
            ),
            (
                CollaborationDbError::Forbidden("permission denied".into()),
                WorkerErrorCode::Unauthorized,
                false,
                "permission denied".to_string(),
            ),
            (
                CollaborationDbError::InvalidState("invalid state".into()),
                WorkerErrorCode::Conflict,
                false,
                "invalid state".to_string(),
            ),
            (
                CollaborationDbError::Internal("internal failure".into()),
                WorkerErrorCode::Conflict,
                false,
                "internal failure".to_string(),
            ),
            (
                CollaborationDbError::Database(database_error),
                WorkerErrorCode::Unavailable,
                true,
                database_message,
            ),
        ];

        for (error, code, retryable, message) in cases {
            let mapped: WorkerError = map_collaboration_db_error(error).into();
            assert_eq!(mapped.code, code);
            assert_eq!(mapped.retryable, retryable);
            assert_eq!(mapped.message, message);
        }
    }

    #[test]
    fn collaboration_internal_error_can_preserve_internal_classification() {
        let mapped: WorkerError = map_collaboration_db_error_preserving_internal(
            CollaborationDbError::Internal("internal failure".into()),
        )
        .into();

        assert_eq!(mapped.code, WorkerErrorCode::Internal);
        assert!(!mapped.retryable);
        assert_eq!(mapped.message, "internal failure");
    }

    #[test]
    fn project_mutation_errors_map_to_the_worker_failure_contract() {
        let database_error = DbErr::Custom("database unavailable".into());
        let database_message = database_error.to_string();
        let cases = [
            (
                ProjectMutationError::InvalidArgument("invalid input".into()),
                WorkerErrorCode::InvalidRequest,
                false,
                "invalid input".to_string(),
            ),
            (
                ProjectMutationError::NotFound("missing project".into()),
                WorkerErrorCode::NotFound,
                false,
                "missing project".to_string(),
            ),
            (
                ProjectMutationError::Forbidden("permission denied".into()),
                WorkerErrorCode::Unauthorized,
                false,
                "permission denied".to_string(),
            ),
            (
                ProjectMutationError::Internal("internal failure".into()),
                WorkerErrorCode::Conflict,
                false,
                "internal failure".to_string(),
            ),
            (
                ProjectMutationError::Database(database_error),
                WorkerErrorCode::Unavailable,
                true,
                database_message,
            ),
        ];

        for (error, code, retryable, message) in cases {
            let mapped: WorkerError = map_project_mutation_error(error).into();
            assert_eq!(mapped.code, code);
            assert_eq!(mapped.retryable, retryable);
            assert_eq!(mapped.message, message);
        }
    }

    #[test]
    fn project_family_errors_map_to_the_worker_failure_contract() {
        let database_error = DbErr::Custom("database unavailable".into());
        let database_message = database_error.to_string();
        let cases = [
            (
                ProjectFamilyDbError::InvalidArgument("invalid input".into()),
                WorkerErrorCode::InvalidRequest,
                false,
                "invalid input".to_string(),
            ),
            (
                ProjectFamilyDbError::NotFound("missing project".into()),
                WorkerErrorCode::NotFound,
                false,
                "missing project".to_string(),
            ),
            (
                ProjectFamilyDbError::Database(database_error),
                WorkerErrorCode::Unavailable,
                true,
                database_message,
            ),
        ];

        for (error, code, retryable, message) in cases {
            let mapped: WorkerError = map_project_family_db_error(error).into();
            assert_eq!(mapped.code, code);
            assert_eq!(mapped.retryable, retryable);
            assert_eq!(mapped.message, message);
        }
    }

    #[test]
    fn workflow_invalid_data_policy_preserves_worker_classification() {
        let error = WorkflowReadError::InvalidData("invalid persisted workflow".into());
        let conflict: WorkerError = map_workflow_read_error_as_conflict(error).into();
        assert_eq!(conflict.code, WorkerErrorCode::Conflict);
        assert!(!conflict.retryable);
        assert_eq!(conflict.message, "invalid persisted workflow");

        let invalid_request: WorkerError = map_workflow_read_error_as_invalid_request(
            WorkflowReadError::InvalidData("invalid persisted workflow".into()),
        )
        .into();
        assert_eq!(invalid_request.code, WorkerErrorCode::InvalidRequest);
        assert!(!invalid_request.retryable);
        assert_eq!(invalid_request.message, "invalid persisted workflow");
    }

    #[test]
    fn workflow_read_errors_preserve_common_error_mapping() {
        let database_error = DbErr::Custom("database unavailable".into());
        let database_message = database_error.to_string();
        let cases = [
            (
                WorkflowReadError::InvalidArgument("invalid input".into()),
                WorkerErrorCode::InvalidRequest,
                false,
                "invalid input".to_string(),
            ),
            (
                WorkflowReadError::NotFound("missing workflow".into()),
                WorkerErrorCode::NotFound,
                false,
                "missing workflow".to_string(),
            ),
            (
                WorkflowReadError::Database(database_error),
                WorkerErrorCode::Unavailable,
                true,
                database_message,
            ),
        ];

        for (error, code, retryable, message) in cases {
            let mapped: WorkerError = map_workflow_read_error_as_conflict(error).into();
            assert_eq!(mapped.code, code);
            assert_eq!(mapped.retryable, retryable);
            assert_eq!(mapped.message, message);
        }
    }

    #[test]
    fn app_errors_map_to_the_chapter_and_video_worker_contract() {
        let cases = [
            (
                AppError::InvalidArgument("invalid input".into()),
                WorkerErrorCode::InvalidRequest,
                false,
                "invalid input".to_string(),
            ),
            (
                AppError::NotFound("missing record".into()),
                WorkerErrorCode::NotFound,
                false,
                "missing record".to_string(),
            ),
            (
                AppError::Forbidden("permission denied".into()),
                WorkerErrorCode::Unauthorized,
                false,
                "permission denied".to_string(),
            ),
            (
                AppError::InvalidState {
                    current: "stale revision".into(),
                    operation: "update record".into(),
                },
                WorkerErrorCode::Conflict,
                false,
                "stale revision: update record".to_string(),
            ),
            (
                AppError::Unauthorized("credentials rejected".into()),
                WorkerErrorCode::Unavailable,
                true,
                "未认证：credentials rejected".to_string(),
            ),
            (
                AppError::ConfigError("invalid configuration".into()),
                WorkerErrorCode::Unavailable,
                true,
                "配置错误：invalid configuration".to_string(),
            ),
        ];

        for (error, code, retryable, message) in cases {
            let mapped: WorkerError = map_app_error(error).into();
            assert_eq!(mapped.code, code);
            assert_eq!(mapped.retryable, retryable);
            assert_eq!(mapped.message, message);
        }
    }

    #[test]
    fn project_family_metadata_read_errors_map_to_the_worker_contract() {
        let invalid = map_project_family_metadata_read_error(
            ProjectFamilyMetadataReadError::InvalidArgument("invalid project".into()),
        );
        let invalid: WorkerError = invalid.into();
        assert_eq!(invalid.code, WorkerErrorCode::InvalidRequest);
        assert!(!invalid.retryable);
        assert_eq!(invalid.message, "invalid project");

        let error = ProjectFamilyMetadataReadError::RootProjectMissing { root_project_id: 7 };
        let message = error.to_string();
        let conflict: WorkerError = map_project_family_metadata_read_error(error).into();
        assert_eq!(conflict.code, WorkerErrorCode::Conflict);
        assert!(!conflict.retryable);
        assert_eq!(conflict.message, message);

        let database_error = DbErr::Custom("database unavailable".into());
        let database_message = database_error.to_string();
        let unavailable: WorkerError =
            map_project_family_metadata_read_error(ProjectFamilyMetadataReadError::Database {
                operation: "read project family",
                source: database_error,
            })
            .into();
        assert_eq!(unavailable.code, WorkerErrorCode::Unavailable);
        assert!(unavailable.retryable);
        assert_eq!(unavailable.message, database_message);
    }

    #[test]
    fn project_progress_read_errors_preserve_invalid_data_classification() {
        let invalid = map_project_progress_read_error(ProjectProgressReadError::InvalidArgument(
            "invalid query".into(),
        ));
        let invalid: WorkerError = invalid.into();
        assert_eq!(invalid.code, WorkerErrorCode::InvalidRequest);
        assert!(!invalid.retryable);
        assert_eq!(invalid.message, "invalid query");

        let error = ProjectProgressReadError::InvalidPersistedStatus {
            table: "novel_project",
            record_id: 7,
            value: "unknown".into(),
        };
        let message = error.to_string();
        let conflict: WorkerError = map_project_progress_read_error(error).into();
        assert_eq!(conflict.code, WorkerErrorCode::Conflict);
        assert!(!conflict.retryable);
        assert_eq!(conflict.message, message);

        let database_error = DbErr::Custom("database unavailable".into());
        let database_message = database_error.to_string();
        let unavailable: WorkerError =
            map_project_progress_read_error(ProjectProgressReadError::Database {
                operation: "read progress",
                source: database_error,
            })
            .into();
        assert_eq!(unavailable.code, WorkerErrorCode::Unavailable);
        assert!(unavailable.retryable);
        assert_eq!(unavailable.message, database_message);
    }

    #[test]
    fn asset_db_errors_map_to_the_worker_contract() {
        let database_error = DbErr::Custom("database unavailable".into());
        let database_message = database_error.to_string();
        let cases = [
            (
                AssetDbError::InvalidArgument("invalid input".into()),
                WorkerErrorCode::InvalidRequest,
                false,
                "invalid input".to_string(),
            ),
            (
                AssetDbError::NotFound("missing asset".into()),
                WorkerErrorCode::NotFound,
                false,
                "missing asset".to_string(),
            ),
            (
                AssetDbError::InvalidState {
                    current: "processing".into(),
                    operation: "delete asset".into(),
                },
                WorkerErrorCode::Conflict,
                false,
                "delete asset: processing".to_string(),
            ),
            (
                AssetDbError::Database(database_error),
                WorkerErrorCode::Unavailable,
                true,
                database_message,
            ),
        ];

        for (error, code, retryable, message) in cases {
            let mapped: WorkerError = map_asset_db_error(error).into();
            assert_eq!(mapped.code, code);
            assert_eq!(mapped.retryable, retryable);
            assert_eq!(mapped.message, message);
        }
    }

    #[test]
    fn database_errors_map_to_retryable_unavailable_failures() {
        let database_error = DbErr::Custom("database unavailable".into());
        let database_message = database_error.to_string();
        let mapped: WorkerError = map_database_error(database_error).into();

        assert_eq!(mapped.code, WorkerErrorCode::Unavailable);
        assert!(mapped.retryable);
        assert_eq!(mapped.message, database_message);
    }

    #[test]
    fn profile_material_db_errors_map_to_the_worker_contract() {
        let database_error = DbErr::Custom("database unavailable".into());
        let database_message = database_error.to_string();
        let cases = [
            (
                ProfileMaterialDbError::InvalidArgument("invalid input".into()),
                WorkerErrorCode::InvalidRequest,
                false,
                "invalid input".to_string(),
            ),
            (
                ProfileMaterialDbError::Database {
                    operation: "load profile materials",
                    source: database_error,
                },
                WorkerErrorCode::Unavailable,
                true,
                database_message,
            ),
        ];

        for (error, code, retryable, message) in cases {
            let mapped: WorkerError = map_profile_material_db_error(error).into();
            assert_eq!(mapped.code, code);
            assert_eq!(mapped.retryable, retryable);
            assert_eq!(mapped.message, message);
        }
    }

    #[test]
    fn scene_profile_db_errors_map_to_the_worker_contract() {
        let database_error = DbErr::Custom("database unavailable".into());
        let database_message = database_error.to_string();
        let cases = [
            (
                SceneProfileDbError::InvalidArgument("invalid input".into()),
                WorkerErrorCode::InvalidRequest,
                false,
                "invalid input".to_string(),
            ),
            (
                SceneProfileDbError::NotFound("missing scene".into()),
                WorkerErrorCode::NotFound,
                false,
                "missing scene".to_string(),
            ),
            (
                SceneProfileDbError::InvalidState {
                    current: "processing".into(),
                    operation: "delete scene".into(),
                },
                WorkerErrorCode::Conflict,
                false,
                "delete scene: processing".to_string(),
            ),
            (
                SceneProfileDbError::Database(database_error),
                WorkerErrorCode::Unavailable,
                true,
                database_message,
            ),
        ];

        for (error, code, retryable, message) in cases {
            let mapped: WorkerError = map_scene_profile_db_error(error).into();
            assert_eq!(mapped.code, code);
            assert_eq!(mapped.retryable, retryable);
            assert_eq!(mapped.message, message);
        }
    }
}
