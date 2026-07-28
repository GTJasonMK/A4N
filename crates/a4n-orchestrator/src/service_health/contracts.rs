use a4n_service_manager::{ManagedSidecarState, ServiceMode};
use a4n_service_protocol::{HealthStatus, ServiceErrorCode, ServiceHealth};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityHealthSnapshot {
    pub prompt: Option<ServiceHealth>,
    pub llm: Option<ServiceHealth>,
    pub embedding: Option<ServiceHealth>,
    pub rag: Option<ServiceHealth>,
    pub image: Option<ServiceHealth>,
    pub video: Option<ServiceHealth>,
    pub import: Option<ServiceHealth>,
    pub export: Option<ServiceHealth>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedServiceHealthSummary {
    pub name: String,
    pub mode: ServiceMode,
    pub service_mode: ServiceMode,
    pub url: Option<String>,
    pub port: Option<u16>,
    pub health_path: String,
    pub binary: Option<String>,
    pub pid: Option<u32>,
    pub pid_status: Option<ManagedServicePidStatus>,
    pub pid_source: Option<String>,
    pub healthy: bool,
    pub status: Option<HealthStatus>,
    pub service_version: Option<String>,
    pub protocol_version: Option<u32>,
    pub capability_version: Option<String>,
    pub restart_count: Option<u32>,
    pub supervisor_state: Option<ManagedSidecarState>,
    pub last_error: Option<String>,
    pub error_code: Option<ServiceErrorCode>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManagedServicePidStatus {
    Observed,
    Missing,
    Stale,
    Invalid,
    Unreadable,
    Unverifiable,
}
