use super::config::{ManagedServiceConfig, RemoteSecurityConfig, RestartPolicy, ServiceMode};
use super::errors::ServiceManagerError;
use super::health::{expected_capability_version, validate_service_health};
use super::manager::{HttpClientFactory, ServiceManager};
use super::process::ManagedSidecarState;
use a4n_service_protocol::{
    EMBEDDING_CAPABILITY_VERSION, EMBEDDING_SERVICE_NAME, EXPORT_RENDER_CAPABILITY_VERSION,
    EXPORT_SERVICE_NAME, HealthStatus, IMAGE_PROVIDER_CAPABILITY_VERSION, IMAGE_SERVICE_NAME,
    IMPORT_SERVICE_NAME, IMPORT_TRANSFORM_CAPABILITY_VERSION, LLM_CHAT_CAPABILITY_VERSION,
    LLM_SERVICE_NAME, PROMPT_RENDER_CAPABILITY_VERSION, PROMPT_SERVICE_NAME, PROTOCOL_VERSION,
    RAG_INDEX_CAPABILITY_VERSION, RAG_SERVICE_NAME, SIDECAR_AUTH_TOKEN_ENV, ServiceErrorCode,
    ServiceHealth, ServiceVersion, VIDEO_PROVIDER_CAPABILITY_VERSION, VIDEO_SERVICE_NAME,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

fn unused_local_addr() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().to_string()
}

fn assert_port_released(addr: &str) {
    let listener = std::net::TcpListener::bind(addr).expect("stopped sidecar should release port");
    drop(listener);
}

struct MtlstestDir(PathBuf);

impl MtlstestDir {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "a4n-service-manager-mtls-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("mTLS test directory should be created");
        Self(path)
    }

    fn file(&self, name: &str) -> String {
        self.0.join(name).display().to_string()
    }
}

impl Drop for MtlstestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct OpenSslServer(Child);

impl Drop for OpenSslServer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn run_openssl(args: Vec<String>) {
    let output = Command::new("openssl")
        .args(&args)
        .output()
        .expect("openssl must be available for the mTLS process test");
    assert!(
        output.status.success(),
        "openssl command failed: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn generate_test_ca(dir: &MtlstestDir) {
    run_openssl(vec![
        "req".into(),
        "-x509".into(),
        "-newkey".into(),
        "rsa:2048".into(),
        "-nodes".into(),
        "-keyout".into(),
        dir.file("ca.key"),
        "-out".into(),
        dir.file("ca.crt"),
        "-subj".into(),
        "/CN=A4N Test CA".into(),
        "-days".into(),
        "1".into(),
    ]);
}

fn generate_test_identity(dir: &MtlstestDir, stem: &str, common_name: &str, usage: &str) {
    let key = dir.file(&format!("{stem}.key"));
    let csr = dir.file(&format!("{stem}.csr"));
    let cert = dir.file(&format!("{stem}.crt"));
    let ext = dir.file(&format!("{stem}.ext"));
    let serial = dir.file(&format!("{stem}.srl"));
    run_openssl(vec![
        "req".into(),
        "-newkey".into(),
        "rsa:2048".into(),
        "-nodes".into(),
        "-keyout".into(),
        key.clone(),
        "-out".into(),
        csr.clone(),
        "-subj".into(),
        format!("/CN={common_name}"),
    ]);
    let extension = if usage == "serverAuth" {
        "subjectAltName=IP:127.0.0.1\nextendedKeyUsage=serverAuth\n"
    } else {
        "extendedKeyUsage=clientAuth\n"
    };
    std::fs::write(&ext, extension).expect("mTLS extension file should be written");
    run_openssl(vec![
        "x509".into(),
        "-req".into(),
        "-in".into(),
        csr,
        "-CA".into(),
        dir.file("ca.crt"),
        "-CAkey".into(),
        dir.file("ca.key"),
        "-CAcreateserial".into(),
        "-CAserial".into(),
        serial,
        "-out".into(),
        cert,
        "-days".into(),
        "1".into(),
        "-extfile".into(),
        ext,
    ]);
}

fn start_mtls_server(dir: &MtlstestDir, addr: &str) -> OpenSslServer {
    let port = addr
        .rsplit_once(':')
        .map(|(_, port)| port)
        .expect("loopback test address should contain a port");
    let child = Command::new("openssl")
        .args([
            "s_server",
            "-accept",
            port,
            "-cert",
            &dir.file("server.crt"),
            "-key",
            &dir.file("server.key"),
            "-CAfile",
            &dir.file("ca.crt"),
            "-Verify",
            "1",
            "-verify_return_error",
            "-www",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("openssl s_server should start");
    OpenSslServer(child)
}

fn request_mtls_status_with_factory(
    runtime: &tokio::runtime::Runtime,
    factory: &HttpClientFactory,
    url: &str,
) -> Result<u16, String> {
    let mut last_error = String::new();
    for _ in 0..40 {
        let result = runtime.block_on(async {
            let client = factory().map_err(|error| error.to_string())?;
            client
                .get(url)
                .send()
                .await
                .map(|response| response.status().as_u16())
                .map_err(|error| error.to_string())
        });
        match result {
            Ok(status) => return Ok(status),
            Err(error) => last_error = error,
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(last_error)
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
        restart_policy: RestartPolicy::default(),
        remote_security: RemoteSecurityConfig::default(),
    };

    let err = ServiceManager::new(vec![config]).expect_err("remote url is required");

    assert!(err.to_string().contains("requires url"));
}

#[test]
fn non_loopback_remote_http_is_rejected() {
    let config = ManagedServiceConfig::remote(PROMPT_SERVICE_NAME, "http://192.0.2.10:47641");
    let error =
        ServiceManager::new(vec![config]).expect_err("non-loopback remote HTTP must be rejected");

    assert!(
        error
            .to_string()
            .contains("must use HTTPS outside loopback")
    );
}

#[test]
fn loopback_remote_http_is_allowed() {
    let mut config = ManagedServiceConfig::remote(PROMPT_SERVICE_NAME, "http://127.0.0.1:47641");
    config.remote_security.auth_token_env = Some(SIDECAR_AUTH_TOKEN_ENV.to_string());
    config.env.insert(
        SIDECAR_AUTH_TOKEN_ENV.to_string(),
        "local-script-token".to_string(),
    );
    ServiceManager::new(vec![config]).expect("loopback remote HTTP should remain available");
}

#[test]
fn non_loopback_remote_https_is_allowed() {
    let mut config =
        ManagedServiceConfig::remote(PROMPT_SERVICE_NAME, "https://prompt.example.test");
    config.remote_security.auth_token_env = Some("A4N_REMOTE_PROMPT_TOKEN".to_string());
    ServiceManager::new(vec![config]).expect("remote HTTPS should be accepted");
}

#[test]
fn non_loopback_remote_rejects_host_credentials() {
    for name in [
        "A4N_DATABASE_URL",
        "A4N_JWT_SECRET",
        "A4N_BOOTSTRAP_ADMIN_PASSWORD",
        "OPENAI_API_KEY",
        "A4N_TEMP_MEDIA_UPLOAD_TOKEN",
        "A4N_VIDEO_REMOTE_TEMP_UPLOAD_TOKEN",
        SIDECAR_AUTH_TOKEN_ENV,
    ] {
        let mut config =
            ManagedServiceConfig::remote(PROMPT_SERVICE_NAME, "https://prompt.example.test");
        config.remote_security.auth_token_env = Some(name.to_string());
        config
            .env
            .insert(name.to_string(), "host-secret-value".to_string());

        let error = ServiceManager::new(vec![config])
            .expect_err("remote deployment credentials must not reuse Host secrets");
        let message = error.to_string();
        assert!(message.contains("independent remote deployment credential"));
        assert!(!message.contains("host-secret-value"));
        assert!(!message.contains("prompt.example.test"));
    }
}

#[test]
fn non_loopback_remote_without_independent_credential_is_rejected() {
    let config = ManagedServiceConfig::remote(PROMPT_SERVICE_NAME, "https://prompt.example.test");
    let error = ServiceManager::new(vec![config]).expect_err("remote auth must be explicit");
    assert!(error.to_string().contains("independent auth_token_env"));
}

#[test]
fn remote_mtls_requires_a_complete_client_identity() {
    let mut config =
        ManagedServiceConfig::remote(PROMPT_SERVICE_NAME, "https://prompt.example.test");
    config.remote_security.client_cert_path = Some("client.pem".into());
    let error = ServiceManager::new(vec![config]).expect_err("mTLS pair must be complete");
    assert!(error.to_string().contains("both client_cert_path"));
}

#[test]
fn remote_http_client_uses_the_declared_credential_without_serializing_it() {
    let mut config =
        ManagedServiceConfig::remote(PROMPT_SERVICE_NAME, "https://prompt.example.test");
    config.remote_security.auth_token_env = Some("A4N_TEST_REMOTE_TOKEN".to_string());
    config.env.insert(
        "A4N_TEST_REMOTE_TOKEN".to_string(),
        "remote-deployment-secret".to_string(),
    );
    let manager = ServiceManager::new(vec![config]).expect("remote config should be valid");
    manager
        .http_client(PROMPT_SERVICE_NAME)
        .expect("remote HTTP client should use system TLS plus the declared credential");
    assert_eq!(
        manager.auth_token(PROMPT_SERVICE_NAME).as_deref(),
        Some("remote-deployment-secret")
    );
    assert!(!format!("{manager:?}").contains("remote-deployment-secret"));
}

#[test]
fn missing_mtls_files_fail_closed_without_exposing_paths() {
    let mut config =
        ManagedServiceConfig::remote(PROMPT_SERVICE_NAME, "https://prompt.example.test");
    config.remote_security.client_cert_path = Some("client-secret-name.pem".into());
    config.remote_security.client_key_path = Some("key-secret-name.pem".into());
    let manager =
        ServiceManager::new(vec![config]).expect("complete mTLS paths pass shape validation");
    let error = manager
        .http_client(PROMPT_SERVICE_NAME)
        .expect_err("unreadable mTLS files must fail closed");
    let message = error.to_string();
    assert!(!message.contains("client-secret-name"));
    assert!(!message.contains("key-secret-name"));
}

#[test]
fn remote_mtls_process_handshake_and_certificate_reload() {
    if Command::new("openssl").arg("version").output().is_err() {
        if std::env::var("A4N_REQUIRE_OPENSSL_MTLS").as_deref() == Ok("1") {
            panic!("openssl is required for the configured remote mTLS process runner");
        }
        eprintln!("skipping remote mTLS process test; openssl is not available");
        return;
    }

    let dir = MtlstestDir::new();
    generate_test_ca(&dir);
    generate_test_identity(&dir, "server", "localhost", "serverAuth");
    generate_test_identity(&dir, "client", "a4n-client", "clientAuth");
    generate_test_identity(&dir, "client-rotated", "a4n-client-rotated", "clientAuth");

    let addr = unused_local_addr();
    let _server = start_mtls_server(&dir, &addr);
    let mut config = ManagedServiceConfig::remote(PROMPT_SERVICE_NAME, format!("https://{addr}"));
    config.remote_security.ca_cert_path = Some(dir.0.join("ca.crt"));
    config.remote_security.client_cert_path = Some(dir.0.join("client.crt"));
    config.remote_security.client_key_path = Some(dir.0.join("client.key"));
    let manager = ServiceManager::new(vec![config]).expect("mTLS remote config should validate");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let factory = manager
        .capability_http_client_factory(PROMPT_SERVICE_NAME)
        .expect("mTLS capability client factory should be available");

    let url = format!("https://{addr}/");
    assert_eq!(
        request_mtls_status_with_factory(&runtime, &factory, &url)
            .expect("client certificate should handshake"),
        200
    );

    std::fs::copy(dir.0.join("client-rotated.crt"), dir.0.join("client.crt"))
        .expect("rotated client certificate should replace the active file");
    std::fs::copy(dir.0.join("client-rotated.key"), dir.0.join("client.key"))
        .expect("rotated client key should replace the active file");
    assert_eq!(
        request_mtls_status_with_factory(&runtime, &factory, &url)
            .expect("manager should reload rotated mTLS identity"),
        200
    );
}

#[test]
fn remote_url_validation_does_not_echo_credentials_or_url() {
    let config = ManagedServiceConfig::remote(
        PROMPT_SERVICE_NAME,
        "https://user:secret@prompt.example.test/path#fragment",
    );
    let error = ServiceManager::new(vec![config]).expect_err("embedded URL credentials are unsafe");
    let message = error.to_string();
    assert!(!message.contains("secret"));
    assert!(!message.contains("prompt.example.test"));
    let debug = format!("{error:?}");
    assert!(!debug.contains("secret"));
    assert!(!debug.contains("prompt.example.test"));
}

#[test]
fn manager_returns_configured_sidecar_auth_token() {
    let mut config = ManagedServiceConfig::sidecar(
        PROMPT_SERVICE_NAME,
        PathBuf::from("target/debug/a4n-prompt-service"),
        "http://127.0.0.1:47641",
    );
    config.env.insert(
        a4n_service_protocol::SIDECAR_AUTH_TOKEN_ENV.to_string(),
        "service-specific-token".to_string(),
    );
    let manager = ServiceManager::new(vec![config]).unwrap();

    assert_eq!(
        manager.auth_token(PROMPT_SERVICE_NAME).as_deref(),
        Some("service-specific-token")
    );
    assert_eq!(manager.auth_token(LLM_SERVICE_NAME), None);
}

#[test]
fn managed_service_debug_exposes_only_environment_keys() {
    let mut config = ManagedServiceConfig::sidecar(
        PROMPT_SERVICE_NAME,
        PathBuf::from("target/debug/a4n-prompt-service"),
        "https://user:secret@sidecar.example.test",
    );
    config.args.push("--credential=argument-secret".to_string());
    config.env.insert(
        SIDECAR_AUTH_TOKEN_ENV.to_string(),
        "service-specific-token".to_string(),
    );
    config.env.insert(
        "HTTPS_PROXY".to_string(),
        "https://proxy-user:proxy-secret@proxy.example.test".to_string(),
    );

    let debug = format!("{config:?}");
    for secret in [
        "service-specific-token",
        "argument-secret",
        "sidecar.example.test",
        "proxy-secret",
        "proxy.example.test",
    ] {
        assert!(!debug.contains(secret));
    }
    assert!(debug.contains(SIDECAR_AUTH_TOKEN_ENV));
    assert!(debug.contains("HTTPS_PROXY"));
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
            compatibility: None,
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
            compatibility: None,
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
            compatibility: None,
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
fn local_sidecar_health_rejects_a_mixed_bundle_build() {
    let config = ManagedServiceConfig::sidecar(
        PROMPT_SERVICE_NAME,
        PathBuf::from("target/debug/a4n-prompt-service"),
        "http://127.0.0.1:47641",
    );
    let health = ServiceHealth {
        service: PROMPT_SERVICE_NAME.to_string(),
        status: HealthStatus::Healthy,
        version: ServiceVersion {
            service_version: "older-sidecar-build".to_string(),
            protocol_version: PROTOCOL_VERSION,
            capability_version: PROMPT_RENDER_CAPABILITY_VERSION.to_string(),
            compatibility: None,
        },
    };

    let error = validate_service_health(&config, &health)
        .expect_err("local bundle must reject mixed service builds");
    assert!(matches!(
        error,
        ServiceManagerError::ServiceBuildMismatch { .. }
    ));
    assert_eq!(
        error.protocol_error_code(),
        ServiceErrorCode::VersionMismatch
    );
}

#[test]
fn remote_health_allows_rolling_build_and_additive_capability_versions() {
    let config = ManagedServiceConfig::remote(PROMPT_SERVICE_NAME, "http://127.0.0.1:47641");
    let health = ServiceHealth {
        service: PROMPT_SERVICE_NAME.to_string(),
        status: HealthStatus::Healthy,
        version: ServiceVersion {
            service_version: "older-remote-build".to_string(),
            protocol_version: PROTOCOL_VERSION,
            capability_version: format!("{PROMPT_RENDER_CAPABILITY_VERSION}.1"),
            compatibility: a4n_service_protocol::service_compatibility(PROMPT_SERVICE_NAME),
        },
    };

    validate_service_health(&config, &health)
        .expect("remote rolling deployment may mix compatible builds");
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

#[test]
fn sidecar_environment_rejects_host_secrets_but_keeps_bootstrap_contract() {
    for key in [
        "A4N_DATABASE_URL",
        "A4N_JWT_SECRET",
        "OPENAI_API_KEY",
        "A4N_TEMP_MEDIA_UPLOAD_TOKEN",
    ] {
        let mut config = ManagedServiceConfig::sidecar(
            PROMPT_SERVICE_NAME,
            PathBuf::from("target/debug/a4n-prompt-service"),
            "http://127.0.0.1:47641",
        );
        config
            .env
            .insert(key.to_string(), "not-forwarded".to_string());
        let error = config
            .validate()
            .expect_err("host secret environment keys must be rejected");
        assert!(error.to_string().contains(key));
    }

    let mut config = ManagedServiceConfig::sidecar(
        PROMPT_SERVICE_NAME,
        PathBuf::from("target/debug/a4n-prompt-service"),
        "http://127.0.0.1:47641",
    );
    config.env.insert(
        SIDECAR_AUTH_TOKEN_ENV.to_string(),
        "manager-token".to_string(),
    );
    config
        .env
        .insert("A4N_DATA_DIR".to_string(), "/tmp/a4n-data".to_string());
    config
        .env
        .insert("HTTPS_PROXY".to_string(), "http://127.0.0.1:1".to_string());
    config
        .validate()
        .expect("bootstrap, path, and proxy variables are allowed");
}

#[tokio::test]
async fn start_sidecar_launches_demo_and_health_reports_exit() {
    let Some(binary) = demo_sidecar_binary_from_env() else {
        return;
    };
    let addr = unused_local_addr();
    let mut config = ManagedServiceConfig::sidecar("demo", binary, format!("http://{addr}"));
    config
        .env
        .insert("A4N_DEMO_SIDECAR_ADDR".to_string(), addr.clone());
    config.env.insert(
        SIDECAR_AUTH_TOKEN_ENV.to_string(),
        "demo-manager-token".to_string(),
    );
    let log_dir = std::env::temp_dir().join(format!(
        "a4n-sidecar-trace-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    config
        .env
        .insert("A4N_LOG_DIR".to_string(), log_dir.display().to_string());
    config.env.insert(
        "A4N_DEMO_SIDECAR_LOG".to_string(),
        "a4n_sidecar_bootstrap=info".to_string(),
    );
    config.startup_timeout_ms = 5_000;
    let manager = ServiceManager::new(vec![config]).unwrap();

    let process = manager.start_sidecar("demo").await.unwrap();
    assert_eq!(process.service_name(), "demo");
    assert!(process.id().is_some());

    let health = manager.health("demo").await.unwrap();
    assert_eq!(health.service, "demo");
    assert_eq!(health.status, HealthStatus::Healthy);
    assert_eq!(health.version.protocol_version, PROTOCOL_VERSION);

    let trace: serde_json::Value = reqwest::Client::new()
        .get(format!("http://{addr}/trace"))
        .header(
            a4n_service_protocol::SIDECAR_AUTH_HEADER,
            "demo-manager-token",
        )
        .header(
            a4n_service_protocol::SIDECAR_REQUEST_ID_HEADER,
            "host-request-1",
        )
        .header(
            a4n_service_protocol::SIDECAR_TRACE_ID_HEADER,
            "host-trace-1",
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(trace["request_id"], "host-request-1");
    assert_eq!(trace["trace_id"], "host-trace-1");

    let metrics: a4n_service_protocol::ServiceRuntimeMetrics = reqwest::Client::new()
        .get(format!("http://{addr}/metrics"))
        .header(
            a4n_service_protocol::SIDECAR_AUTH_HEADER,
            "demo-manager-token",
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(metrics.service, "demo");
    assert_eq!(metrics.restart_count, 0);
    assert!(metrics.requests_total >= 2);
    assert_eq!(metrics.operations["/trace"].requests_total, 1);

    let persisted_log = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let log = std::fs::read_dir(log_dir.join("services"))
                .unwrap()
                .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
                .collect::<String>();
            if log.contains("host-request-1") && log.contains("host-trace-1") {
                break log;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("sidecar log should be readable while the process is alive");
    assert!(persisted_log.contains("host-request-1"));
    assert!(persisted_log.contains("host-trace-1"));

    process.stop().await.unwrap();

    let err = manager
        .health("demo")
        .await
        .expect_err("stopped demo sidecar should not stay healthy");
    assert_eq!(
        err.protocol_error_code(),
        ServiceErrorCode::ServiceUnavailable
    );
    let _ = std::fs::remove_dir_all(log_dir);
}

#[tokio::test]
async fn supervisor_restarts_once_and_does_not_replay_in_flight_request() {
    let Some(binary) = demo_sidecar_binary_from_env() else {
        return;
    };
    let addr = unused_local_addr();
    let base_url = format!("http://{addr}");
    let counter = std::env::temp_dir().join(format!(
        "a4n-sidecar-counter-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let token = "demo-supervisor-token";
    let mut config = ManagedServiceConfig::sidecar("demo", binary, base_url.clone());
    config
        .env
        .insert("A4N_DEMO_SIDECAR_ADDR".to_string(), addr.clone());
    config
        .env
        .insert(SIDECAR_AUTH_TOKEN_ENV.to_string(), token.to_string());
    config.env.insert(
        "A4N_DEMO_SIDECAR_COUNTER_FILE".to_string(),
        counter.display().to_string(),
    );
    config.restart_policy = RestartPolicy {
        max_restarts: 2,
        initial_backoff_ms: 20,
        max_backoff_ms: 40,
    };
    let manager = ServiceManager::new(vec![config]).unwrap();
    let process = manager.start_sidecar("demo").await.unwrap();
    let client = reqwest::Client::new();

    let in_flight = tokio::spawn({
        let client = client.clone();
        let base_url = base_url.clone();
        async move {
            client
                .post(base_url.clone() + "/counted-slow")
                .header(a4n_service_protocol::SIDECAR_AUTH_HEADER, token)
                .send()
                .await
        }
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    let _ = client
        .post(base_url + "/crash")
        .header(a4n_service_protocol::SIDECAR_AUTH_HEADER, token)
        .send()
        .await;
    let response = in_flight.await.unwrap();
    assert!(
        response.is_err(),
        "crashed in-flight request must fail explicitly"
    );

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(status) = manager.process_status("demo").await.unwrap()
                && status.state == ManagedSidecarState::Running
                && status.restart_count == 1
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("supervisor should restart the demo sidecar");

    let lines = std::fs::read_to_string(&counter)
        .expect("in-flight request should leave one durable counter entry");
    assert_eq!(
        lines.lines().count(),
        1,
        "manager must not replay the request"
    );
    process.stop().await.unwrap();
    let _ = std::fs::remove_file(counter);
}

#[tokio::test]
async fn supervisor_opens_circuit_after_restart_budget_is_exhausted() {
    let Some(binary) = demo_sidecar_binary_from_env() else {
        return;
    };
    let addr = unused_local_addr();
    let base_url = format!("http://{addr}");
    let token = "demo-circuit-token";
    let mut config = ManagedServiceConfig::sidecar("demo", binary, base_url.clone());
    config.env.insert("A4N_DEMO_SIDECAR_ADDR".to_string(), addr);
    config
        .env
        .insert(SIDECAR_AUTH_TOKEN_ENV.to_string(), token.to_string());
    config.restart_policy = RestartPolicy {
        max_restarts: 1,
        initial_backoff_ms: 10,
        max_backoff_ms: 10,
    };
    let manager = ServiceManager::new(vec![config]).unwrap();
    let process = manager.start_sidecar("demo").await.unwrap();
    let client = reqwest::Client::new();

    let _ = client
        .post(base_url.clone() + "/crash")
        .header(a4n_service_protocol::SIDECAR_AUTH_HEADER, token)
        .send()
        .await;
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if manager
                .process_status("demo")
                .await
                .unwrap()
                .is_some_and(|status| {
                    status.state == ManagedSidecarState::Running && status.restart_count == 1
                })
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("first restart should become healthy");
    let _ = client
        .post(base_url + "/crash")
        .header(a4n_service_protocol::SIDECAR_AUTH_HEADER, token)
        .send()
        .await;

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if manager
                .process_status("demo")
                .await
                .unwrap()
                .is_some_and(|status| status.state == ManagedSidecarState::CircuitOpen)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("restart budget should open the circuit");

    let error = process
        .stop()
        .await
        .expect_err("stopping a circuit-open supervisor must expose its failure");
    assert!(error.to_string().contains("restart circuit opened"));
}

#[tokio::test]
async fn two_managers_own_distinct_sidecars_and_stop_independently() {
    let Some(binary) = demo_sidecar_binary_from_env() else {
        return;
    };

    let first_addr = unused_local_addr();
    let mut first_config =
        ManagedServiceConfig::sidecar("demo", binary.clone(), format!("http://{first_addr}"));
    first_config
        .env
        .insert("A4N_DEMO_SIDECAR_ADDR".to_string(), first_addr.clone());
    first_config.env.insert(
        SIDECAR_AUTH_TOKEN_ENV.to_string(),
        "first-manager-token".to_string(),
    );

    let first_manager = ServiceManager::new(vec![first_config]).unwrap();
    let first_process = first_manager.start_sidecar("demo").await.unwrap();

    let second_addr = unused_local_addr();
    assert_ne!(first_addr, second_addr);
    let mut second_config =
        ManagedServiceConfig::sidecar("demo", binary, format!("http://{second_addr}"));
    second_config
        .env
        .insert("A4N_DEMO_SIDECAR_ADDR".to_string(), second_addr.clone());
    second_config.env.insert(
        SIDECAR_AUTH_TOKEN_ENV.to_string(),
        "second-manager-token".to_string(),
    );

    let second_manager = ServiceManager::new(vec![second_config]).unwrap();
    let second_process = second_manager.start_sidecar("demo").await.unwrap();

    assert_ne!(first_process.id(), second_process.id());
    assert_ne!(
        first_manager.auth_token("demo"),
        second_manager.auth_token("demo")
    );
    assert_eq!(first_manager.health("demo").await.unwrap().service, "demo");
    assert_eq!(second_manager.health("demo").await.unwrap().service, "demo");

    first_process.stop().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(first_manager.health("demo").await.is_err());
    assert_eq!(second_manager.health("demo").await.unwrap().service, "demo");
    assert_port_released(&first_addr);

    second_process.stop().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(second_manager.health("demo").await.is_err());
    assert_port_released(&second_addr);
}
