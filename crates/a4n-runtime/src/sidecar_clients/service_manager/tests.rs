use std::collections::HashSet;

use a4n_core::config::app_config::AppConfig;
use a4n_service_manager::ServiceMode;
use a4n_service_protocol::service_catalog;

use crate::state::RuntimeProfile;

use super::configs::{reserve_managed_loopback_addr, runtime_service_configs};
use super::specs::parse_sidecar_addr_env;

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
