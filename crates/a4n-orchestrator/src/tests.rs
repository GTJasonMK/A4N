use std::path::PathBuf;

use a4n_service_manager::{ManagedServiceConfig, ServiceManagerError, ServiceMode};
use a4n_service_protocol::{HealthStatus, ServiceHealth};

use super::service_health::service_health_summary;
use super::{freeform_generated_reset_error_to_app_error, project_delete_error_to_app_error};

#[test]
fn project_delete_error_keeps_typed_app_error_mapping() {
    let missing = project_delete_error_to_app_error(
        a4n_db::project_delete::ProjectDeleteDbError::ProjectNotFound { project_id: 9 },
    );
    assert!(matches!(
        missing,
        a4n_core::error::AppError::NotFound(message)
            if message == "项目不存在: project_id=9"
    ));

    let database =
        project_delete_error_to_app_error(a4n_db::project_delete::ProjectDeleteDbError::Database {
            source: sea_orm::DbErr::Custom("delete failed".to_string()),
        });
    assert!(matches!(
        database,
        a4n_core::error::AppError::Database(sea_orm::DbErr::Custom(message))
            if message == "delete failed"
    ));

    let cleanup_database = project_delete_error_to_app_error(
        a4n_db::project_delete::ProjectDeleteDbError::ProjectCleanup {
            source: a4n_db::project::ProjectCleanupDbError::Database {
                source: sea_orm::DbErr::Custom("cleanup failed".to_string()),
            },
        },
    );
    assert!(matches!(
        cleanup_database,
        a4n_core::error::AppError::Database(sea_orm::DbErr::Custom(message))
            if message == "cleanup failed"
    ));
}

#[test]
fn freeform_generated_reset_error_keeps_typed_app_error_mapping() {
    let missing = freeform_generated_reset_error_to_app_error(
        a4n_db::project::FreeformGeneratedResetDbError::ProjectNotFound { project_id: 11 },
    );
    assert!(matches!(
        missing,
        a4n_core::error::AppError::NotFound(message) if message == "项目不存在: 11"
    ));

    let unsupported = freeform_generated_reset_error_to_app_error(
        a4n_db::project::FreeformGeneratedResetDbError::UnsupportedProject,
    );
    assert!(matches!(
        unsupported,
        a4n_core::error::AppError::InvalidArgument(message)
            if message == "仅自由画布视频项目支持该生成数据重置入口"
    ));

    let database = freeform_generated_reset_error_to_app_error(
        a4n_db::project::FreeformGeneratedResetDbError::Database {
            source: sea_orm::DbErr::Custom("reset failed".to_string()),
        },
    );
    assert!(matches!(
        database,
        a4n_core::error::AppError::Database(sea_orm::DbErr::Custom(message))
            if message == "reset failed"
    ));
}

#[test]
fn disabled_service_summary_keeps_mode_and_error_message() {
    let config = ManagedServiceConfig::disabled("rag");
    let summary = service_health_summary(
        config,
        Err(ServiceManagerError::CapabilityDisabled("rag".to_string())),
        None,
    );

    assert_eq!(summary.name, "rag");
    assert_eq!(summary.mode, ServiceMode::Disabled);
    assert!(!summary.healthy);
    assert_eq!(
        summary.error_message.as_deref(),
        Some("capability disabled: rag")
    );
}

#[test]
fn sidecar_service_summary_uses_manager_owned_pid() {
    let health = ServiceHealth {
        service: "image".to_string(),
        status: HealthStatus::Healthy,
        version: a4n_service_protocol::ServiceVersion {
            service_version: "test".to_string(),
            protocol_version: a4n_service_protocol::PROTOCOL_VERSION,
            capability_version: "test".to_string(),
        },
    };

    let summary = service_health_summary(
        ManagedServiceConfig::sidecar(
            "image",
            PathBuf::from("target/debug/a4n-image-service"),
            "http://127.0.0.1:47645",
        ),
        Ok(health),
        Some(std::process::id()),
    );

    assert_eq!(summary.pid, Some(std::process::id()));
}
