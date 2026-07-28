use std::path::PathBuf;

use a4n_service_manager::{
    ManagedServiceConfig, ManagedSidecarState, ManagedSidecarStatus, ServiceManagerError,
    ServiceMode,
};
use a4n_service_protocol::{HealthStatus, ServiceErrorCode, ServiceHealth};

use super::service_health::{redact_service_url, service_health_summary, service_url_port};
use super::{
    ManagedServicePidStatus, freeform_generated_reset_error_to_app_error,
    project_delete_error_to_app_error,
};

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
        a4n_db::project::FreeformGeneratedResetDbError::UnsupportedProject {
            project_kind: "text".to_string(),
            source_mode: "self".to_string(),
        },
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
fn service_url_summary_redacts_userinfo_query_and_fragment() {
    let url = "http://user:secret@127.0.0.1:47647/health?token=abc#frag";

    assert_eq!(redact_service_url(url), "http://127.0.0.1:47647/health");
}

#[test]
fn service_url_port_extracts_common_sidecar_ports() {
    assert_eq!(service_url_port("http://127.0.0.1:47648"), Some(47648));
    assert_eq!(service_url_port("http://[::1]:47649/healthz"), Some(47649));
    assert_eq!(service_url_port("http://[::1]/healthz"), None);
}

#[test]
fn disabled_service_summary_keeps_mode_and_error_code() {
    let config = ManagedServiceConfig::disabled("rag");
    let summary = service_health_summary(
        config,
        Err(ServiceManagerError::CapabilityDisabled("rag".to_string())),
        None,
        None,
    );

    assert_eq!(summary.name, "rag");
    assert_eq!(summary.mode, ServiceMode::Disabled);
    assert_eq!(summary.service_mode, ServiceMode::Disabled);
    assert!(!summary.healthy);
    assert_eq!(
        summary.error_code,
        Some(ServiceErrorCode::CapabilityDisabled)
    );
    assert_eq!(
        summary.error_message.as_deref(),
        Some("capability disabled: rag")
    );
}

#[test]
fn remote_service_summary_reads_observed_pid_file() {
    let pid_dir = unique_temp_dir("a4n_orchestrator_pid_observed");
    std::fs::create_dir_all(&pid_dir).expect("create pid dir");
    std::fs::write(pid_dir.join("llm.pid"), std::process::id().to_string())
        .expect("write pid file");
    let health = ServiceHealth {
        service: "llm".to_string(),
        status: HealthStatus::Healthy,
        version: a4n_service_protocol::ServiceVersion {
            service_version: "test".to_string(),
            protocol_version: a4n_service_protocol::PROTOCOL_VERSION,
            capability_version: "test".to_string(),
            compatibility: None,
        },
    };

    let summary = service_health_summary(
        ManagedServiceConfig::remote("llm", "http://127.0.0.1:47642"),
        Ok(health),
        Some(&pid_dir),
        None,
    );

    assert_eq!(summary.pid, Some(std::process::id()));
    assert_eq!(summary.pid_status, Some(ManagedServicePidStatus::Observed));
    assert_eq!(summary.pid_source.as_deref(), Some("pid_file"));

    let _ = std::fs::remove_dir_all(pid_dir);
}

#[test]
fn remote_service_summary_marks_missing_pid_file() {
    let pid_dir = unique_temp_dir("a4n_orchestrator_pid_missing");
    std::fs::create_dir_all(&pid_dir).expect("create pid dir");

    let summary = service_health_summary(
        ManagedServiceConfig::remote("rag", "http://127.0.0.1:47644"),
        Err(ServiceManagerError::CapabilityDisabled("rag".to_string())),
        Some(&pid_dir),
        None,
    );

    assert_eq!(summary.pid, None);
    assert_eq!(summary.pid_status, Some(ManagedServicePidStatus::Missing));
    assert_eq!(summary.pid_source.as_deref(), Some("pid_file"));

    let _ = std::fs::remove_dir_all(pid_dir);
}

#[test]
fn embedded_service_summary_ignores_pid_file() {
    let pid_dir = unique_temp_dir("a4n_orchestrator_pid_embedded");
    std::fs::create_dir_all(&pid_dir).expect("create pid dir");
    std::fs::write(pid_dir.join("prompt.pid"), std::process::id().to_string())
        .expect("write pid file");

    let summary = service_health_summary(
        ManagedServiceConfig::embedded("prompt"),
        Ok(ServiceHealth {
            service: "prompt".to_string(),
            status: HealthStatus::Healthy,
            version: a4n_service_protocol::ServiceVersion {
                service_version: "test".to_string(),
                protocol_version: a4n_service_protocol::PROTOCOL_VERSION,
                capability_version: "embedded".to_string(),
                compatibility: None,
            },
        }),
        Some(&pid_dir),
        None,
    );

    assert_eq!(summary.pid, None);
    assert_eq!(summary.pid_status, None);
    assert_eq!(summary.pid_source, None);

    let _ = std::fs::remove_dir_all(pid_dir);
}

#[test]
fn sidecar_service_summary_prefers_manager_owned_pid() {
    let pid_dir = unique_temp_dir("a4n_orchestrator_manager_pid");
    std::fs::create_dir_all(&pid_dir).expect("create pid dir");
    std::fs::write(pid_dir.join("image.pid"), "not-a-pid").expect("write pid file");
    let health = ServiceHealth {
        service: "image".to_string(),
        status: HealthStatus::Healthy,
        version: a4n_service_protocol::ServiceVersion {
            service_version: "test".to_string(),
            protocol_version: a4n_service_protocol::PROTOCOL_VERSION,
            capability_version: "test".to_string(),
            compatibility: None,
        },
    };

    let summary = service_health_summary(
        ManagedServiceConfig::sidecar(
            "image",
            PathBuf::from("target/debug/a4n-image-service"),
            "http://127.0.0.1:47645",
        ),
        Ok(health),
        Some(&pid_dir),
        Some(&ManagedSidecarStatus {
            service: "image".to_string(),
            state: ManagedSidecarState::Running,
            pid: Some(std::process::id()),
            restart_count: 1,
            next_restart_in_ms: None,
            last_error: Some("previous crash".to_string()),
        }),
    );

    assert_eq!(summary.pid, Some(std::process::id()));
    assert_eq!(summary.pid_status, Some(ManagedServicePidStatus::Observed));
    assert_eq!(summary.pid_source.as_deref(), Some("manager_owned"));
    assert_eq!(summary.restart_count, Some(1));
    assert_eq!(summary.last_error.as_deref(), Some("previous crash"));

    let _ = std::fs::remove_dir_all(pid_dir);
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{prefix}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
