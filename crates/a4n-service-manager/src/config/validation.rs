use super::contracts::{ManagedServiceConfig, ServiceMode};
use crate::errors::ServiceManagerError;
use crate::health::expected_capability_version;

impl ManagedServiceConfig {
    pub fn validate(&self) -> Result<(), ServiceManagerError> {
        if self.name.trim().is_empty() {
            return Err(ServiceManagerError::InvalidConfig(
                "service name cannot be blank".to_string(),
            ));
        }
        if !self.health_path.starts_with('/') {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "service {} health_path must start with /",
                self.name
            )));
        }

        match self.mode {
            ServiceMode::Embedded => self.validate_registered_local_mode("embedded"),
            ServiceMode::Disabled => self.validate_registered_local_mode("disabled"),
            ServiceMode::Remote => self.validate_remote(),
            ServiceMode::Sidecar => self.validate_sidecar(),
        }
    }

    fn validate_registered_local_mode(&self, mode: &str) -> Result<(), ServiceManagerError> {
        if expected_capability_version(&self.name).is_none() {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "{mode} service {} has no registered capability version",
                self.name
            )));
        }
        Ok(())
    }

    fn validate_remote(&self) -> Result<(), ServiceManagerError> {
        if self.url.as_deref().map(str::trim).unwrap_or("").is_empty() {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "remote service {} requires url",
                self.name
            )));
        }
        Ok(())
    }

    fn validate_sidecar(&self) -> Result<(), ServiceManagerError> {
        if self.binary.is_none() {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "sidecar service {} requires binary",
                self.name
            )));
        }
        if self.url.as_deref().map(str::trim).unwrap_or("").is_empty() {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "sidecar service {} requires url",
                self.name
            )));
        }
        Ok(())
    }
}
