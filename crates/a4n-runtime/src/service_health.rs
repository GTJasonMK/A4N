use std::sync::Arc;

use a4n_api_contract::service_health::{
    ManagedServiceHealthSummary, ManagedServiceMode, ServiceHealthApplicationPort,
};
use a4n_service_manager::{ManagedServiceConfig, ServiceManager, ServiceManagerError, ServiceMode};
use a4n_service_protocol::ServiceHealth;

/// Runtime owns managed-process health observation; business orchestration does
/// not need to be involved in this lifecycle-only query.
pub(crate) struct ServiceHealthApplication {
    service_manager: Arc<ServiceManager>,
}

impl ServiceHealthApplication {
    pub(crate) fn new(service_manager: Arc<ServiceManager>) -> Self {
        Self { service_manager }
    }
}

#[async_trait::async_trait]
impl ServiceHealthApplicationPort for ServiceHealthApplication {
    async fn summaries(&self) -> Result<Vec<ManagedServiceHealthSummary>, a4n_error::AppError> {
        let results = futures::future::join_all(self.service_manager.configs().into_iter().map(
            |config| async {
                let (health, managed_status) = futures::join!(
                    self.service_manager.health(&config.name),
                    self.service_manager.process_status(&config.name),
                );
                let managed_status = managed_status.map_err(|error| {
                    a4n_error::AppError::Internal(anyhow::anyhow!(error.to_string()))
                })?;
                Ok::<_, a4n_error::AppError>(service_health_summary(
                    config,
                    health,
                    managed_status.and_then(|status| status.pid),
                ))
            },
        ))
        .await;

        results.into_iter().collect()
    }
}

fn service_health_summary(
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

fn service_manager_error_message(error: &ServiceManagerError) -> String {
    match error {
        ServiceManagerError::UnknownService(service) => format!("unknown service: {service}"),
        ServiceManagerError::InvalidConfig(message) => format!("invalid service config: {message}"),
        ServiceManagerError::CapabilityDisabled(service) => {
            format!("capability disabled: {service}")
        }
        ServiceManagerError::HealthCheckFailed { service, .. } => {
            format!("service {service} health check failed")
        }
        ServiceManagerError::HttpStatus { service, status } => {
            format!("service {service} returned HTTP status {status}")
        }
        ServiceManagerError::Unhealthy { service, status } => {
            format!("service {service} is unhealthy: {status:?}")
        }
        ServiceManagerError::ProtocolMismatch {
            service,
            expected,
            actual,
        } => format!("service {service} protocol mismatch: expected {expected}, actual {actual}"),
        ServiceManagerError::CapabilityMismatch {
            service,
            expected,
            actual,
        } => format!("service {service} capability mismatch: expected {expected}, actual {actual}"),
        ServiceManagerError::ServiceIdentityMismatch { expected, actual } => {
            format!("service identity mismatch: expected {expected}, actual {actual}")
        }
        ServiceManagerError::ServiceBuildMismatch {
            service,
            expected,
            actual,
        } => format!("service {service} build mismatch: expected {expected}, actual {actual}"),
        ServiceManagerError::Timeout {
            service,
            timeout_ms,
        } => format!("service {service} startup timed out after {timeout_ms}ms"),
        ServiceManagerError::Exited { service, status } => {
            format!("service {service} exited before becoming healthy: status {status:?}")
        }
        ServiceManagerError::ProcessStatusFailed { service, .. } => {
            format!("service {service} process status failed")
        }
        ServiceManagerError::SpawnFailed { service, .. } => {
            format!("service {service} spawn failed")
        }
        ServiceManagerError::ReleaseManifestInvalid(_) => {
            "service release manifest validation failed".to_string()
        }
        ServiceManagerError::BinaryIntegrityFailed { service, .. } => {
            format!("service {service} binary integrity validation failed")
        }
        ServiceManagerError::StopFailed { service, .. } => {
            format!("service {service} stop failed")
        }
        ServiceManagerError::StartupCleanupFailed {
            service, startup, ..
        } => format!(
            "service {service} startup failed and child cleanup also failed: {}",
            service_manager_error_message(startup.as_ref())
        ),
        ServiceManagerError::SupervisorTaskFailed { service, .. } => {
            format!("service {service} supervisor task failed")
        }
        ServiceManagerError::RestartLimitExceeded {
            service, restarts, ..
        } => format!("service {service} restart limit exceeded after {restarts} attempts"),
    }
}

#[cfg(test)]
mod tests {
    use super::service_health_summary;
    use a4n_api_contract::service_health::ManagedServiceMode;
    use a4n_service_manager::{ManagedServiceConfig, ServiceManagerError};
    use a4n_service_protocol::{HealthStatus, PROTOCOL_VERSION, ServiceHealth, ServiceVersion};
    use std::path::PathBuf;

    #[test]
    fn disabled_service_summary_keeps_mode_and_error_message() {
        let summary = service_health_summary(
            ManagedServiceConfig::disabled("rag"),
            Err(ServiceManagerError::CapabilityDisabled("rag".to_string())),
            None,
        );

        assert_eq!(summary.name, "rag");
        assert_eq!(summary.mode, ManagedServiceMode::Disabled);
        assert!(!summary.healthy);
        assert_eq!(
            summary.error_message.as_deref(),
            Some("capability disabled: rag")
        );
    }

    #[test]
    fn sidecar_service_summary_uses_manager_owned_pid() {
        let summary = service_health_summary(
            ManagedServiceConfig::sidecar(
                "image",
                PathBuf::from("target/debug/a4n-image-service"),
                "http://127.0.0.1:47645",
            ),
            Ok(ServiceHealth {
                service: "image".to_string(),
                status: HealthStatus::Healthy,
                version: ServiceVersion {
                    service_version: "test".to_string(),
                    protocol_version: PROTOCOL_VERSION,
                    capability_version: "test".to_string(),
                },
            }),
            Some(std::process::id()),
        );

        assert_eq!(summary.pid, Some(std::process::id()));
    }
}
