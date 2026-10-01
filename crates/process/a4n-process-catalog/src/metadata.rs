use crate::contracts::{ProcessMetadata, ServiceCatalogEntry, StatefulWorkerCatalogEntry};

impl From<&ServiceCatalogEntry> for ProcessMetadata {
    fn from(entry: &ServiceCatalogEntry) -> Self {
        Self {
            name: entry.name.clone(),
            kind: entry.kind,
            binary: entry.binary.clone(),
            capability_version: Some(entry.capability_version.clone()),
            contract_version: Some(entry.contract_version.clone()),
            addr_env: entry.addr_env.clone(),
            url_env: Some(entry.url_env.clone()),
            default_addr: entry.default_addr.clone(),
            local_only: entry.local_only,
            data_scope: entry.data_scope.clone(),
            read_scope: Vec::new(),
            write_scope: Vec::new(),
            capabilities: Vec::new(),
            dependencies: entry.dependencies.clone(),
            migration: entry.migration.clone(),
        }
    }
}

impl From<&StatefulWorkerCatalogEntry> for ProcessMetadata {
    fn from(entry: &StatefulWorkerCatalogEntry) -> Self {
        Self {
            name: entry.name.clone(),
            kind: entry.kind,
            binary: entry.binary.clone(),
            capability_version: Some(entry.capability_version.clone()),
            contract_version: Some(entry.contract_version.clone()),
            addr_env: entry.addr_env.clone(),
            url_env: entry.url_env.clone(),
            default_addr: entry.default_addr.clone(),
            local_only: entry.local_only,
            data_scope: Vec::new(),
            read_scope: entry.read_scope.clone(),
            write_scope: entry.write_scope.clone(),
            capabilities: entry.capabilities.clone(),
            dependencies: entry.dependencies.clone(),
            migration: entry.migration.clone(),
        }
    }
}
