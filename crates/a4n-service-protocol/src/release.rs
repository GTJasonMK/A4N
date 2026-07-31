use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReleaseManifest {
    pub schema_version: u32,
    pub package: String,
    pub version: String,
    pub target: String,
    pub protocol_version: u32,
    pub binaries: Vec<ReleaseBinaryManifest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ReleaseToolManifest>,
    pub runtime_files: Vec<ReleaseRuntimeFileManifest>,
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

impl ReleaseManifest {
    pub fn binary(&self, name: &str) -> Option<&ReleaseBinaryManifest> {
        self.binaries.iter().find(|entry| entry.name == name)
    }
}
