use serde::{Deserialize, Serialize};

pub const RELEASE_MANIFEST_SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReleaseManifest {
    pub schema_version: u32,
    pub package: String,
    pub version: String,
    pub target: String,
    pub protocol_version: u32,
    /// 本次发布实际携带的 capability service 子集；空值兼容旧的全量 manifest。
    #[serde(default)]
    pub services: Vec<String>,
    pub binaries: Vec<ReleaseBinaryManifest>,
    pub tools: Vec<ReleaseToolManifest>,
    pub runtime_files: Vec<ReleaseRuntimeFileManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReleaseToolCatalogEntry {
    pub package: String,
    pub binary: String,
}

pub fn release_tool_catalog() -> Result<Vec<ReleaseToolCatalogEntry>, serde_json::Error> {
    serde_json::from_str(include_str!("../release-tools.json"))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReleaseBinaryManifest {
    pub name: String,
    pub service: String,
    pub path: String,
    pub version: String,
    pub target: String,
    pub sha256: String,
    pub protocol_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReleaseRuntimeFileManifest {
    pub name: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReleaseToolManifest {
    pub name: String,
    pub path: String,
    pub version: String,
    pub target: String,
    pub sha256: String,
}
