use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessKind {
    CapabilitySidecar,
    StatefulWorker,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationCompatibility {
    #[serde(default)]
    pub current: Option<String>,
    #[serde(default)]
    pub min_compatible: Option<String>,
    #[serde(default)]
    pub max_compatible: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessMetadata {
    pub name: String,
    pub kind: ProcessKind,
    pub binary: String,
    #[serde(default)]
    pub capability_version: Option<String>,
    #[serde(default)]
    pub contract_version: Option<String>,
    pub addr_env: String,
    #[serde(default)]
    pub url_env: Option<String>,
    pub default_addr: String,
    #[serde(default)]
    pub local_only: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_scope: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub read_scope: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub write_scope: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub migration: MigrationCompatibility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceCatalogEntry {
    pub name: String,
    pub binary: String,
    pub capability_version: String,
    pub addr_env: String,
    pub url_env: String,
    pub default_addr: String,
    #[serde(default)]
    pub local_only: bool,
    pub kind: ProcessKind,
    pub contract_version: String,
    pub data_scope: Vec<String>,
    pub dependencies: Vec<String>,
    pub migration: MigrationCompatibility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatefulWorkerCatalogEntry {
    pub name: String,
    pub kind: ProcessKind,
    pub binary: String,
    pub contract_version: String,
    pub capability_version: String,
    pub addr_env: String,
    #[serde(default)]
    pub url_env: Option<String>,
    pub default_addr: String,
    #[serde(default)]
    pub local_only: bool,
    pub read_scope: Vec<String>,
    pub write_scope: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub migration: MigrationCompatibility,
}
