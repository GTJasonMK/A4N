use a4n_service_manager::{ManagedServiceConfig, ServiceManagerError, ServiceMode};
use a4n_service_protocol::ServiceHealth;

use super::errors::service_manager_error_message;
use a4n_api_contract::service_health::{ManagedServiceHealthSummary, ManagedServiceMode};

pub(crate) fn service_health_summary(
    config: ManagedServiceConfig,
    health: Result<ServiceHealth, ServiceManagerError>,
    managed_pid: Option<u32>,
) -> ManagedServiceHealthSummary {
    let url = config.url.clone();
    let binary = config
        .binary
        .as_ref()
        .map(|path| path.display().to_string());
    let (healthy, service_version, protocol_version, capability_version, error_message) =
        match health {
            Ok(health) => (
                true,
                Some(health.version.service_version),
                Some(health.version.protocol_version),
                Some(health.version.capability_version),
                None,
            ),
            Err(err) => (
                false,
                None,
                None,
                None,
                Some(service_manager_error_message(&err)),
            ),
        };
    ManagedServiceHealthSummary {
        name: config.name,
        mode: match config.mode {
            ServiceMode::Embedded => ManagedServiceMode::Embedded,
            ServiceMode::Sidecar => ManagedServiceMode::Sidecar,
            ServiceMode::Remote => ManagedServiceMode::Remote,
            ServiceMode::Disabled => ManagedServiceMode::Disabled,
        },
        url,
        binary,
        pid: managed_pid,
        healthy,
        service_version,
        protocol_version,
        capability_version,
        error_message,
    }
}
