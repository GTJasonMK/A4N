mod release;

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::OnceLock;

pub use release::{
    RELEASE_MANIFEST_SCHEMA_VERSION, ReleaseBinaryManifest, ReleaseManifest,
    ReleaseRuntimeFileManifest, ReleaseToolCatalogEntry, ReleaseToolManifest, release_tool_catalog,
};

/// 进程管理、开发脚本和发布包共享的 sidecar 清单。
///
/// 清单文件是唯一的机器可读来源；运行时和检查脚本都应从这里派生，
/// 不再各自维护服务名、端口和环境变量的平行列表。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceCatalogEntry {
    pub name: String,
    pub binary: String,
    pub capability_version: String,
    pub addr_env: String,
    pub url_env: String,
    pub default_addr: String,
}

pub fn service_catalog() -> Result<Vec<ServiceCatalogEntry>, serde_json::Error> {
    serde_json::from_str(include_str!("../service-catalog.json"))
}

fn embedded_service_catalog() -> &'static [ServiceCatalogEntry] {
    static CATALOG: OnceLock<Vec<ServiceCatalogEntry>> = OnceLock::new();
    CATALOG
        .get_or_init(|| service_catalog().expect("embedded service catalog must be valid"))
        .as_slice()
}

pub fn service_catalog_entry(name: &str) -> Option<ServiceCatalogEntry> {
    service_catalog_entry_ref(name).cloned()
}

pub fn service_catalog_entry_ref(name: &str) -> Option<&'static ServiceCatalogEntry> {
    embedded_service_catalog()
        .iter()
        .find(|entry| entry.name == name)
}

pub fn service_catalog_entry_for_capability_version(
    capability_version: &str,
) -> Option<ServiceCatalogEntry> {
    service_catalog_entry_for_capability_version_ref(capability_version).cloned()
}

pub fn service_catalog_entry_for_capability_version_ref(
    capability_version: &str,
) -> Option<&'static ServiceCatalogEntry> {
    embedded_service_catalog()
        .iter()
        .find(|entry| entry.capability_version == capability_version)
}

pub fn parse_service_socket_addr(key: &str, value: &str) -> Result<SocketAddr, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{key} 不能为空"));
    }
    value
        .parse()
        .map_err(|error| format!("{key} 必须是有效监听地址: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{
        parse_service_socket_addr, service_catalog, service_catalog_entry_for_capability_version,
    };

    #[test]
    fn catalog_has_unique_process_identity_and_environment_keys() {
        let entries = service_catalog().expect("embedded service catalog must be valid");
        assert_eq!(entries.len(), 8);
        for field in [
            entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            entries
                .iter()
                .map(|entry| entry.binary.as_str())
                .collect::<Vec<_>>(),
            entries
                .iter()
                .map(|entry| entry.addr_env.as_str())
                .collect::<Vec<_>>(),
            entries
                .iter()
                .map(|entry| entry.url_env.as_str())
                .collect::<Vec<_>>(),
        ] {
            let unique = field.iter().collect::<std::collections::HashSet<_>>();
            assert_eq!(unique.len(), field.len());
        }
        assert!(entries.iter().all(|entry| {
            !entry.capability_version.trim().is_empty()
                && entry.default_addr.parse::<std::net::SocketAddr>().is_ok()
        }));
    }

    #[test]
    fn service_socket_addr_has_one_strict_parser() {
        assert_eq!(
            parse_service_socket_addr("A4N_SERVICE_ADDR", " 127.0.0.1:47641 ").unwrap(),
            "127.0.0.1:47641".parse().unwrap()
        );
        assert!(parse_service_socket_addr("A4N_SERVICE_ADDR", " ").is_err());
        let error = parse_service_socket_addr("A4N_SERVICE_ADDR", "token=sidecar-secret")
            .expect_err("invalid address must fail");
        assert!(error.contains("A4N_SERVICE_ADDR 必须是有效监听地址"));
        assert!(!error.contains("sidecar-secret"));
    }

    #[test]
    fn catalog_resolves_capability_version_to_one_service() {
        assert_eq!(
            service_catalog_entry_for_capability_version("export-render-v3")
                .map(|entry| entry.name),
            Some("export".to_string())
        );
        assert!(service_catalog_entry_for_capability_version("unknown-v1").is_none());
    }
}
