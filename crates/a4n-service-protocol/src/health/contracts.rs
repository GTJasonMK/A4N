use serde::{Deserialize, Serialize};

use crate::ServiceCompatibility;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceHealth {
    pub service: String,
    pub status: HealthStatus,
    pub version: ServiceVersion,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceVersion {
    pub service_version: String,
    pub protocol_version: u32,
    pub capability_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<ServiceCompatibility>,
}
