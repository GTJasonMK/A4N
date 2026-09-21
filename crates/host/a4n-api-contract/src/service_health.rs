use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Deployment mode reported by the service manager.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManagedServiceMode {
    Embedded,
    Sidecar,
    Remote,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ManagedServiceHealthSummary {
    pub name: String,
    pub mode: ManagedServiceMode,
    pub url: Option<String>,
    pub binary: Option<String>,
    pub pid: Option<u32>,
    pub healthy: bool,
    pub service_version: Option<String>,
    pub protocol_version: Option<u32>,
    pub capability_version: Option<String>,
    pub error_message: Option<String>,
}

/// Host-facing health query for managed capabilities.
#[async_trait::async_trait]
pub trait ServiceHealthApplicationPort: Send + Sync {
    async fn summaries(&self) -> Result<Vec<ManagedServiceHealthSummary>, a4n_error::AppError>;
}
