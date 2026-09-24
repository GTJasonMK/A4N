use std::collections::HashSet;
use std::path::PathBuf;

use a4n_config::app_config::AppConfig;
use a4n_export_contract::EXPORT_SERVICE_NAME;
use a4n_process_catalog::{RELEASE_MANIFEST_SCHEMA_VERSION, ReleaseManifest, service_catalog};
use a4n_service_manager::{ManagedServiceConfig, ServiceMode};
use a4n_service_protocol::PROTOCOL_VERSION;

use crate::state::RuntimeProfile;

use super::configs::{
    managed_sidecar_service, reserve_managed_loopback_addr, runtime_service_configs,
    runtime_sidecar_service_configs, selected_service_names,
};
use super::specs::{SIDECAR_BIN_DIR_ENV, parse_sidecar_addr_env};

#[test]
fn full_profile_keeps_all_capabilities_embedded() {
    let configs = runtime_service_configs(RuntimeProfile::Full, &AppConfig::default())
        .expect("full profile configs");
    let catalog = service_catalog().expect("embedded sidecar service catalog");

    assert_eq!(
        configs
            .iter()
            .map(|config| config.name.as_str())
            .collect::<Vec<_>>(),
        catalog
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>()
    );
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
fn release_manifest_enables_only_the_selected_export_service() {
    let (_manifest_dir, manifest_path) = write_release_manifest(&[EXPORT_SERVICE_NAME]);
    let selected_services =
        selected_service_names(Some(&manifest_path)).expect("release manifest services");
    let configs = runtime_sidecar_service_configs(&selected_services, |spec| {
        Ok(ManagedServiceConfig::sidecar(
            spec.name,
            PathBuf::from("unused-sidecar"),
            "http://127.0.0.1:47648",
        ))
    })
    .expect("selected sidecar configs");

    let enabled_services = configs
        .iter()
        .filter(|config| config.mode != ServiceMode::Disabled)
        .map(|config| config.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(enabled_services, vec![EXPORT_SERVICE_NAME]);
}

#[test]
fn selected_export_without_binary_fails_sidecar_configuration() {
    let (_manifest_dir, manifest_path) = write_release_manifest(&[EXPORT_SERVICE_NAME]);
    let selected_services =
        selected_service_names(Some(&manifest_path)).expect("release manifest services");
    let mut reserved_addrs = HashSet::new();
    let error = runtime_sidecar_service_configs(&selected_services, |spec| {
        managed_sidecar_service(spec, &AppConfig::default(), &mut reserved_addrs, None)
    })
    .expect_err("a selected sidecar without its binary must fail");

    assert!(error.contains("需要可执行 sidecar 二进制"));
    assert!(error.contains("a4n-export-service"));
    assert!(error.contains(SIDECAR_BIN_DIR_ENV));
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

fn write_release_manifest(services: &[&str]) -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().expect("temporary manifest directory");
    let path = directory.path().join("release-manifest.json");
    let manifest = ReleaseManifest {
        schema_version: RELEASE_MANIFEST_SCHEMA_VERSION,
        package: "a4n-test".to_string(),
        version: "test".to_string(),
        target: "test-target".to_string(),
        protocol_version: PROTOCOL_VERSION,
        services: services
            .iter()
            .map(|service| (*service).to_string())
            .collect(),
        binaries: Vec::new(),
        tools: Vec::new(),
        runtime_files: Vec::new(),
    };
    let contents = serde_json::to_vec(&manifest).expect("serialize release manifest");
    std::fs::write(&path, contents).expect("write release manifest");
    (directory, path)
}
