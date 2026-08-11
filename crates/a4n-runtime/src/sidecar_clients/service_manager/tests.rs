use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use a4n_core::config::app_config::AppConfig;
use a4n_service_manager::{RemoteSecurityConfig, ServiceMode};
use a4n_service_protocol::SIDECAR_AUTH_TOKEN_ENV;

use crate::state::RuntimeProfile;

use super::binaries::{
    current_tauri_target_triples, sidecar_binary_path_in_dirs, sidecar_platform_executable_name,
    tauri_sidecar_executable_name,
};
use super::configs::{
    build_remote_service, remote_security_env_prefix, reserve_managed_loopback_addr,
    runtime_service_configs,
};
use super::specs::{parse_managed_sidecars_env, parse_sidecar_addr_env, parse_sidecar_url_env};

struct TestSidecarDir {
    path: PathBuf,
}

impl TestSidecarDir {
    fn new(label: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "a4n-sidecar-binary-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp sidecar dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn sidecar(&self, executable_name: String) -> PathBuf {
        self.path.join(executable_name)
    }
}

impl Drop for TestSidecarDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn touch(path: &Path) {
    fs::write(path, b"").expect("create sidecar binary placeholder");
}

#[test]
fn full_profile_keeps_all_capabilities_embedded() {
    let configs = runtime_service_configs(RuntimeProfile::Full, &AppConfig::default())
        .expect("full profile configs");

    assert_eq!(configs.len(), 8);
    assert!(
        configs
            .iter()
            .all(|config| config.mode == ServiceMode::Embedded)
    );
}

#[test]
fn minimal_profile_only_embeds_lightweight_capabilities() {
    let configs = runtime_service_configs(RuntimeProfile::Minimal, &AppConfig::default())
        .expect("minimal profile configs");

    for config in configs {
        let expected = if matches!(config.name.as_str(), "prompt" | "import" | "export") {
            ServiceMode::Embedded
        } else {
            ServiceMode::Disabled
        };
        assert_eq!(config.mode, expected, "service={}", config.name);
    }
}

#[test]
fn managed_sidecars_env_is_strictly_parsed() {
    assert!(!parse_managed_sidecars_env(None).unwrap());
    for value in ["1", "true", "YES", " on "] {
        assert!(parse_managed_sidecars_env(Some(value)).unwrap(), "{value}");
    }
    for value in ["0", "false", "NO", " off "] {
        assert!(!parse_managed_sidecars_env(Some(value)).unwrap(), "{value}");
    }
    let error = parse_managed_sidecars_env(Some("ture"))
        .expect_err("invalid managed-sidecar mode must not silently select remote mode");
    assert!(error.contains("A4N_MANAGED_SIDECARS 必须是"));
}

#[test]
fn managed_sidecars_reserve_distinct_loopback_ports() {
    let mut reserved = HashSet::new();
    let first = reserve_managed_loopback_addr(&mut reserved).expect("reserve first sidecar port");
    let second = reserve_managed_loopback_addr(&mut reserved).expect("reserve second sidecar port");

    assert!(first.ip().is_loopback());
    assert!(second.ip().is_loopback());
    assert_ne!(first, second);
}

#[test]
fn remote_service_uses_service_scoped_security_instead_of_local_bootstrap() {
    assert_eq!(remote_security_env_prefix("prompt"), "A4N_REMOTE_PROMPT");
    let config = build_remote_service(
        "prompt",
        "https://prompt.example.test".to_string(),
        "/ready".to_string(),
        RemoteSecurityConfig {
            auth_token_env: Some("A4N_REMOTE_PROMPT_TOKEN".to_string()),
            ..Default::default()
        },
        Some("local-script-token"),
    );

    assert_eq!(
        config.remote_security.auth_token_env.as_deref(),
        Some("A4N_REMOTE_PROMPT_TOKEN")
    );
    assert_eq!(config.health_path, "/ready");
    assert!(!config.env.contains_key(SIDECAR_AUTH_TOKEN_ENV));
    config
        .validate()
        .expect("service-scoped remote credentials should pass manager validation");

    let local = build_remote_service(
        "prompt",
        "http://127.0.0.1:47641".to_string(),
        "/healthz".to_string(),
        RemoteSecurityConfig::default(),
        Some("local-script-token"),
    );
    assert_eq!(
        local.remote_security.auth_token_env.as_deref(),
        Some(SIDECAR_AUTH_TOKEN_ENV)
    );
    assert_eq!(
        local.env.get(SIDECAR_AUTH_TOKEN_ENV).map(String::as_str),
        Some("local-script-token")
    );
    local
        .validate()
        .expect("loopback script-owned sidecars should keep the local bootstrap token");
}

#[test]
fn sidecar_address_env_is_strictly_parsed() {
    assert_eq!(
        parse_sidecar_addr_env("A4N_TEST_ADDR", None, "127.0.0.1:47641").unwrap(),
        "127.0.0.1:47641"
    );
    assert!(
        parse_sidecar_addr_env("A4N_TEST_ADDR", Some(""), "127.0.0.1:47641")
            .unwrap_err()
            .contains("不能为空")
    );
    assert!(
        parse_sidecar_addr_env("A4N_TEST_ADDR", Some("localhost:47641"), "127.0.0.1:47641")
            .unwrap_err()
            .contains("有效监听地址")
    );
}

#[test]
fn sidecar_url_env_rejects_empty_non_http_credentials_query_and_fragment() {
    assert_eq!(parse_sidecar_url_env("A4N_TEST_URL", None).unwrap(), None);
    assert!(
        parse_sidecar_url_env("A4N_TEST_URL", Some("  ".to_string()))
            .unwrap_err()
            .contains("不能为空")
    );
    assert!(
        parse_sidecar_url_env("A4N_TEST_URL", Some("ftp://127.0.0.1:21".to_string()))
            .unwrap_err()
            .contains("http:// 或 https://")
    );
    assert!(
        parse_sidecar_url_env(
            "A4N_TEST_URL",
            Some("http://user:secret@127.0.0.1:47641".to_string()),
        )
        .unwrap_err()
        .contains("用户名或密码")
    );
    assert!(
        parse_sidecar_url_env(
            "A4N_TEST_URL",
            Some("http://127.0.0.1:47641/api#ignored".to_string()),
        )
        .unwrap_err()
        .contains("不能包含查询参数或 fragment")
    );
    assert!(
        parse_sidecar_url_env(
            "A4N_TEST_URL",
            Some("http://127.0.0.1:47641/api?scope=ignored".to_string()),
        )
        .unwrap_err()
        .contains("不能包含查询参数或 fragment")
    );
    assert_eq!(
        parse_sidecar_url_env(
            "A4N_TEST_URL",
            Some(" http://127.0.0.1:47641/ ".to_string()),
        )
        .unwrap(),
        Some("http://127.0.0.1:47641".to_string())
    );
}

#[test]
fn binary_discovery_prefers_unsuffixed_package_name() {
    let dir = TestSidecarDir::new("prefers-unsuffixed");
    let base = dir.sidecar(sidecar_platform_executable_name("a4n-prompt-service"));
    let tauri = dir.sidecar(tauri_sidecar_executable_name(
        "a4n-prompt-service",
        "custom-target",
    ));
    touch(&tauri);
    touch(&base);

    assert_eq!(
        sidecar_binary_path_in_dirs("prompt", [dir.path()]),
        Some(base)
    );
}

#[test]
fn binary_discovery_accepts_one_tauri_suffixed_binary() {
    let dir = TestSidecarDir::new("unique-tauri");
    let tauri = dir.sidecar(tauri_sidecar_executable_name(
        "a4n-llm-gateway",
        "custom-target",
    ));
    touch(&tauri);

    assert_eq!(
        sidecar_binary_path_in_dirs("llm", [dir.path()]),
        Some(tauri)
    );
}

#[test]
fn binary_discovery_rejects_ambiguous_tauri_suffixes() {
    let dir = TestSidecarDir::new("ambiguous-tauri");
    touch(&dir.sidecar(tauri_sidecar_executable_name(
        "a4n-rag-service",
        "custom-target-a",
    )));
    touch(&dir.sidecar(tauri_sidecar_executable_name(
        "a4n-rag-service",
        "custom-target-b",
    )));

    assert_eq!(sidecar_binary_path_in_dirs("rag", [dir.path()]), None);
}

#[test]
fn binary_discovery_uses_current_target_before_ambiguous_scan() {
    let Some(target) = current_tauri_target_triples().first().copied() else {
        return;
    };
    let dir = TestSidecarDir::new("current-target");
    let current = dir.sidecar(tauri_sidecar_executable_name("a4n-image-service", target));
    let other = dir.sidecar(tauri_sidecar_executable_name(
        "a4n-image-service",
        "custom-target",
    ));
    touch(&other);
    touch(&current);

    assert_eq!(
        sidecar_binary_path_in_dirs("image", [dir.path()]),
        Some(current)
    );
}
