use a4n_service_manager::{ManagedServiceConfig, ServiceManagerError};
use a4n_service_protocol::ServiceHealth;

use super::contracts::ManagedServiceHealthSummary;
use super::errors::service_manager_error_message;

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
        mode: config.mode,
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
