use super::config::{ManagedServiceConfig, ServiceMode};
use super::errors::ServiceManagerError;
use super::health::{expected_capability_version, validate_service_health};
use super::manager::ServiceManager;
use a4n_service_protocol::{
    EMBEDDING_CAPABILITY_VERSION, EMBEDDING_SERVICE_NAME, EXPORT_RENDER_CAPABILITY_VERSION,
    EXPORT_SERVICE_NAME, HealthStatus, IMAGE_PROVIDER_CAPABILITY_VERSION, IMAGE_SERVICE_NAME,
    IMPORT_SERVICE_NAME, IMPORT_TRANSFORM_CAPABILITY_VERSION, LLM_CHAT_CAPABILITY_VERSION,
    LLM_SERVICE_NAME, PROMPT_RENDER_CAPABILITY_VERSION, PROMPT_SERVICE_NAME, PROTOCOL_VERSION,
    RAG_INDEX_CAPABILITY_VERSION, RAG_SERVICE_NAME, ServiceErrorCode, ServiceHealth,
    ServiceVersion, VIDEO_PROVIDER_CAPABILITY_VERSION, VIDEO_SERVICE_NAME,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

fn unused_local_addr() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().to_string()
}

fn demo_sidecar_binary_from_env() -> Option<PathBuf> {
    let Ok(binary) = std::env::var("A4N_DEMO_SIDECAR_BIN") else {
        eprintln!("skipping demo sidecar start test; A4N_DEMO_SIDECAR_BIN is not set");
        return None;
    };
    let path = PathBuf::from(binary);
    if path.is_absolute() {
        return Some(path.canonicalize().unwrap_or(path));
    }

    let current_dir = std::env::current_dir().ok();
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|crates_dir| crates_dir.parent())
        .map(PathBuf::from);

    for base in [current_dir, Some(manifest_dir), workspace_root]
        .into_iter()
        .flatten()
    {
        let candidate = base.join(&path);
        if candidate.exists() {
            return Some(candidate.canonicalize().unwrap_or(candidate));
        }
    }

    Some(path)
}

#[test]
fn disabled_health_maps_to_capability_disabled_code() {
    let manager =
        ServiceManager::new(vec![ManagedServiceConfig::disabled(RAG_SERVICE_NAME)]).unwrap();
    let err = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(manager.health(RAG_SERVICE_NAME))
        .expect_err("disabled service should fail");

    assert!(matches!(err, ServiceManagerError::CapabilityDisabled(_)));
    assert_eq!(
        err.protocol_error_code(),
        ServiceErrorCode::CapabilityDisabled
    );
}

#[test]
fn remote_service_requires_url() {
    let config = ManagedServiceConfig {
        name: LLM_SERVICE_NAME.to_string(),
        mode: ServiceMode::Remote,
        binary: None,
        args: Vec::new(),
        url: None,
        health_path: "/healthz".to_string(),
        env: HashMap::new(),
        startup_timeout_ms: 1000,
    };

    let err = ServiceManager::new(vec![config]).expect_err("remote url is required");

    assert!(err.to_string().contains("requires url"));
}

#[test]
fn known_embedded_health_uses_protocol_and_capability_version() {
    let manager =
        ServiceManager::new(vec![ManagedServiceConfig::embedded(PROMPT_SERVICE_NAME)]).unwrap();
    let health = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(manager.health(PROMPT_SERVICE_NAME))
        .unwrap();

    assert_eq!(health.status, HealthStatus::Healthy);
    assert_eq!(health.version.protocol_version, PROTOCOL_VERSION);
    assert_eq!(
        health.version.capability_version,
        PROMPT_RENDER_CAPABILITY_VERSION
    );
}

#[test]
fn unknown_embedded_config_requires_registered_capability_version() {
    let error = ServiceManager::new(vec![ManagedServiceConfig::embedded("demo")])
        .expect_err("unknown embedded service must fail during configuration");

    assert!(matches!(
        error,
        ServiceManagerError::InvalidConfig(message)
            if message.contains("demo") && message.contains("registered capability version")
    ));
}

#[test]
fn known_services_have_expected_capability_versions() {
    assert_eq!(
        expected_capability_version(PROMPT_SERVICE_NAME),
        Some(PROMPT_RENDER_CAPABILITY_VERSION)
    );
    assert_eq!(
        expected_capability_version(LLM_SERVICE_NAME),
        Some(LLM_CHAT_CAPABILITY_VERSION)
    );
    assert_eq!(
        expected_capability_version(EMBEDDING_SERVICE_NAME),
        Some(EMBEDDING_CAPABILITY_VERSION)
    );
    assert_eq!(
        expected_capability_version(RAG_SERVICE_NAME),
        Some(RAG_INDEX_CAPABILITY_VERSION)
    );
    assert_eq!(
        expected_capability_version(IMAGE_SERVICE_NAME),
        Some(IMAGE_PROVIDER_CAPABILITY_VERSION)
    );
    assert_eq!(
        expected_capability_version(VIDEO_SERVICE_NAME),
        Some(VIDEO_PROVIDER_CAPABILITY_VERSION)
    );
    assert_eq!(
        expected_capability_version(IMPORT_SERVICE_NAME),
        Some(IMPORT_TRANSFORM_CAPABILITY_VERSION)
    );
    assert_eq!(
        expected_capability_version(EXPORT_SERVICE_NAME),
        Some(EXPORT_RENDER_CAPABILITY_VERSION)
    );
    assert_eq!(expected_capability_version("demo"), None);
}

#[test]
fn capability_mismatch_maps_to_version_mismatch_code() {
    let err = ServiceManagerError::CapabilityMismatch {
        service: PROMPT_SERVICE_NAME.to_string(),
        expected: PROMPT_RENDER_CAPABILITY_VERSION.to_string(),
        actual: "prompt-render-v0".to_string(),
    };

    assert_eq!(err.protocol_error_code(), ServiceErrorCode::VersionMismatch);
    assert!(err.to_string().contains("capability mismatch"));
}

#[test]
fn health_validation_rejects_known_service_capability_mismatch() {
    let config = ManagedServiceConfig::remote(PROMPT_SERVICE_NAME, "http://127.0.0.1:47641");
    let health = ServiceHealth {
        service: PROMPT_SERVICE_NAME.to_string(),
        status: HealthStatus::Healthy,
        version: ServiceVersion {
            service_version: "0.1.0".to_string(),
            protocol_version: PROTOCOL_VERSION,
            capability_version: "prompt-render-v0".to_string(),
        },
    };

    let err = validate_service_health(&config, &health).expect_err("capability mismatch");

    assert!(matches!(
        err,
        ServiceManagerError::CapabilityMismatch { .. }
    ));
    assert_eq!(err.protocol_error_code(), ServiceErrorCode::VersionMismatch);
}

#[test]
fn health_validation_rejects_service_identity_mismatch() {
    let config = ManagedServiceConfig::remote(PROMPT_SERVICE_NAME, "http://127.0.0.1:47641");
    let health = ServiceHealth {
        service: LLM_SERVICE_NAME.to_string(),
        status: HealthStatus::Healthy,
        version: ServiceVersion {
            service_version: "0.1.0".to_string(),
            protocol_version: PROTOCOL_VERSION,
            capability_version: PROMPT_RENDER_CAPABILITY_VERSION.to_string(),
        },
    };

    let err = validate_service_health(&config, &health).expect_err("identity mismatch");

    assert!(matches!(
        err,
        ServiceManagerError::ServiceIdentityMismatch { .. }
    ));
    assert_eq!(err.protocol_error_code(), ServiceErrorCode::VersionMismatch);
    assert!(err.to_string().contains("identity mismatch"));
}

#[test]
fn health_validation_rejects_unhealthy_status() {
    let config = ManagedServiceConfig::remote(PROMPT_SERVICE_NAME, "http://127.0.0.1:47641");
    let health = ServiceHealth {
        service: PROMPT_SERVICE_NAME.to_string(),
        status: HealthStatus::Degraded,
        version: ServiceVersion {
            service_version: "0.1.0".to_string(),
            protocol_version: PROTOCOL_VERSION,
            capability_version: PROMPT_RENDER_CAPABILITY_VERSION.to_string(),
        },
    };

    let err = validate_service_health(&config, &health).expect_err("unhealthy status");

    assert!(matches!(err, ServiceManagerError::Unhealthy { .. }));
    assert_eq!(
        err.protocol_error_code(),
        ServiceErrorCode::ServiceUnavailable
    );
}

#[test]
fn sidecar_constructor_sets_required_fields() {
    let config = ManagedServiceConfig::sidecar(
        "demo",
        PathBuf::from("target/debug/a4n-demo-sidecar"),
        "http://127.0.0.1:47631",
    );

    assert_eq!(config.mode, ServiceMode::Sidecar);
    assert_eq!(config.health_path, "/healthz");
    assert!(config.binary.is_some());
    assert!(config.url.as_deref().is_some_and(|url| !url.is_empty()));
    config.validate().unwrap();
}

#[tokio::test]
async fn start_sidecar_launches_demo_and_health_reports_exit() {
    let Some(binary) = demo_sidecar_binary_from_env() else {
        return;
    };
    let addr = unused_local_addr();
    let mut config = ManagedServiceConfig::sidecar("demo", binary, format!("http://{addr}"));
    config.env.insert("A4N_DEMO_SIDECAR_ADDR".to_string(), addr);
    config.startup_timeout_ms = 5_000;
    let manager = ServiceManager::new(vec![config]).unwrap();

    let process = manager.start_sidecar("demo").await.unwrap();
    assert_eq!(process.service_name(), "demo");
    assert!(process.id().is_some());

    let health = manager.health("demo").await.unwrap();
    assert_eq!(health.service, "demo");
    assert_eq!(health.status, HealthStatus::Healthy);
    assert_eq!(health.version.protocol_version, PROTOCOL_VERSION);

    process.stop().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let err = manager
        .health("demo")
        .await
        .expect_err("stopped demo sidecar should not stay healthy");
    assert_eq!(
        err.protocol_error_code(),
        ServiceErrorCode::ServiceUnavailable
    );
}
