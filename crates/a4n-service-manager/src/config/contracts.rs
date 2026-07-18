use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceMode {
    Embedded,
    Sidecar,
    Remote,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedServiceConfig {
    pub name: String,
    pub mode: ServiceMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default = "default_health_path")]
    pub health_path: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    #[serde(default = "default_startup_timeout_ms")]
    pub startup_timeout_ms: u64,
}

pub(super) fn default_health_path() -> String {
    "/healthz".to_string()
}

pub(super) fn default_startup_timeout_ms() -> u64 {
    5_000
}
