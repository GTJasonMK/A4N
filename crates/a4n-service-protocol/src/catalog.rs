use serde::{Deserialize, Serialize};

/// 进程管理、开发脚本和发布包共享的 sidecar 清单。
///
/// 清单文件是唯一的机器可读来源；运行时和检查脚本都应从这里派生，
/// 不再各自维护服务名、端口和环境变量的平行列表。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceCatalogEntry {
    pub name: String,
    pub binary: String,
    pub capability_version: String,
    pub addr_env: String,
    pub url_env: String,
    pub health_path: String,
    pub default_addr: String,
    pub default_url: String,
}

pub fn service_catalog() -> Result<Vec<ServiceCatalogEntry>, serde_json::Error> {
    serde_json::from_str(include_str!("../service-catalog.json"))
}

pub fn service_catalog_entry(name: &str) -> Option<ServiceCatalogEntry> {
    service_catalog()
        .ok()?
        .into_iter()
        .find(|entry| entry.name == name)
}

#[cfg(test)]
mod tests {
    use super::service_catalog;
    use crate::{expected_capability_version_for_service, registry};

    #[test]
    fn catalog_matches_protocol_registry() {
        let entries = service_catalog().expect("embedded service catalog must be valid");
        assert_eq!(entries.len(), 8);
        for entry in entries {
            assert_eq!(
                expected_capability_version_for_service(&entry.name),
                Some(entry.capability_version.as_str()),
                "catalog capability version drifted for {}",
                entry.name
            );
            assert!(registry::expected_capability_label_for_service(&entry.name).is_some());
            assert!(entry.default_url.ends_with(&entry.default_addr));
        }
    }
}
