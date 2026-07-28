use std::path::Path;

use a4n_service_manager::{ManagedServiceConfig, ManagedSidecarStatus, ServiceManagerError};
use a4n_service_protocol::{HealthStatus, ServiceHealth};

use super::contracts::ManagedServiceHealthSummary;
use super::errors::service_manager_error_message;
use super::pid::service_pid_observation;
use super::url::service_config_metadata;

pub(crate) fn service_health_summary(
    config: ManagedServiceConfig,
    health: Result<ServiceHealth, ServiceManagerError>,
    service_pid_dir: Option<&Path>,
    managed_status: Option<&ManagedSidecarStatus>,
) -> ManagedServiceHealthSummary {
    let metadata = service_config_metadata(&config);
    let pid = service_pid_observation(
        &config,
        service_pid_dir,
        managed_status.and_then(|status| status.pid),
    );
    let restart_count = managed_status.map(|status| status.restart_count);
    let supervisor_state = managed_status.map(|status| status.state);
    let last_error = managed_status.and_then(|status| status.last_error.clone());
    match health {
        Ok(health) => ManagedServiceHealthSummary {
            name: config.name,
            mode: config.mode,
            service_mode: config.mode,
            url: metadata.url,
            port: metadata.port,
            health_path: metadata.health_path,
            binary: metadata.binary,
            pid: pid.pid,
            pid_status: pid.status,
            pid_source: pid.source,
            healthy: health.status == HealthStatus::Healthy,
            status: Some(health.status),
            service_version: Some(health.version.service_version),
            protocol_version: Some(health.version.protocol_version),
            capability_version: Some(health.version.capability_version),
            restart_count,
            supervisor_state,
            last_error,
            error_code: None,
            error_message: None,
        },
        Err(err) => ManagedServiceHealthSummary {
            name: config.name,
            mode: config.mode,
            service_mode: config.mode,
            url: metadata.url,
            port: metadata.port,
            health_path: metadata.health_path,
            binary: metadata.binary,
            pid: pid.pid,
            pid_status: pid.status,
            pid_source: pid.source,
            healthy: false,
            status: None,
            service_version: None,
            protocol_version: None,
            capability_version: None,
            restart_count,
            supervisor_state,
            last_error,
            error_code: Some(err.protocol_error_code()),
            error_message: Some(service_manager_error_message(&err)),
        },
    }
}
