use a4n_service_manager::ServiceMode;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ManagedServiceHealthSummary {
    pub name: String,
    pub mode: ServiceMode,
    pub url: Option<String>,
    pub binary: Option<String>,
    pub pid: Option<u32>,
    pub healthy: bool,
    pub service_version: Option<String>,
    pub protocol_version: Option<u32>,
    pub capability_version: Option<String>,
    pub error_message: Option<String>,
}
