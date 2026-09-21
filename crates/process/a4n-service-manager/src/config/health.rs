use a4n_process_catalog::service_catalog_entry;
use a4n_service_protocol::{HealthStatus, PROTOCOL_VERSION, ServiceHealth, ServiceVersion};

use super::contracts::ManagedServiceConfig;
use crate::errors::ServiceManagerError;

impl ManagedServiceConfig {
    pub(crate) fn endpoint_url(&self, path: &str) -> Result<String, ServiceManagerError> {
        let base = self.url.as_deref().ok_or_else(|| {
            ServiceManagerError::InvalidConfig(format!("service {} has no url", self.name))
        })?;
        Ok(format!("{}{path}", base.trim_end_matches('/')))
    }

    pub(crate) fn embedded_health(&self) -> Result<ServiceHealth, ServiceManagerError> {
        let capability_version = service_catalog_entry(&self.name)
            .map(|entry| entry.capability_version)
            .ok_or_else(|| {
                ServiceManagerError::InvalidConfig(format!(
                    "embedded service {} has no registered capability version",
                    self.name
                ))
            })?;
        Ok(ServiceHealth {
            service: self.name.clone(),
            status: HealthStatus::Healthy,
            version: ServiceVersion {
                service_version: env!("CARGO_PKG_VERSION").to_string(),
                protocol_version: PROTOCOL_VERSION,
                capability_version,
            },
        })
    }
}
