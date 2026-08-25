use a4n_service_manager::ServiceMode;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ManagedServiceHealthSummary {
    pub(crate) name: String,
    pub(crate) mode: ServiceMode,
    pub(crate) url: Option<String>,
    pub(crate) binary: Option<String>,
    pub(crate) pid: Option<u32>,
    pub(crate) healthy: bool,
    pub(crate) service_version: Option<String>,
    pub(crate) protocol_version: Option<u32>,
    pub(crate) capability_version: Option<String>,
    pub(crate) error_message: Option<String>,
}
