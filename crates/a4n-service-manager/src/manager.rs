use super::config::{ManagedServiceConfig, ServiceMode};
use super::errors::ServiceManagerError;
use super::health::http_health;
use super::process::ManagedSidecarProcess;
use a4n_service_protocol::ServiceHealth;
use std::collections::HashMap;
use std::time::Duration;
use tokio::process::Command;

#[derive(Debug)]
pub struct ServiceManager {
    configs: HashMap<String, ManagedServiceConfig>,
    client: reqwest::Client,
}

impl ServiceManager {
    pub fn new(configs: Vec<ManagedServiceConfig>) -> Result<Self, ServiceManagerError> {
        let mut by_name = HashMap::new();
        for config in configs {
            config.validate()?;
            if by_name.insert(config.name.clone(), config).is_some() {
                return Err(ServiceManagerError::InvalidConfig(
                    "duplicate service name".to_string(),
                ));
            }
        }
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(1_000))
            .timeout(Duration::from_millis(3_000))
            .build()
            .map_err(|err| {
                ServiceManagerError::InvalidConfig(format!(
                    "service health HTTP client init failed: {err}"
                ))
            })?;
        Ok(Self {
            configs: by_name,
            client,
        })
    }

    pub fn config(&self, name: &str) -> Result<&ManagedServiceConfig, ServiceManagerError> {
        self.configs
            .get(name)
            .ok_or_else(|| ServiceManagerError::UnknownService(name.to_string()))
    }

    pub fn configs(&self) -> Vec<ManagedServiceConfig> {
        let mut configs = self.configs.values().cloned().collect::<Vec<_>>();
        configs.sort_by(|left, right| left.name.cmp(&right.name));
        configs
    }

    pub async fn health(&self, name: &str) -> Result<ServiceHealth, ServiceManagerError> {
        let config = self.config(name)?;
        match config.mode {
            ServiceMode::Embedded => config.embedded_health(),
            ServiceMode::Disabled => {
                Err(ServiceManagerError::CapabilityDisabled(config.name.clone()))
            }
            ServiceMode::Remote | ServiceMode::Sidecar => self.http_health(config).await,
        }
    }

    pub async fn start_sidecar(
        &self,
        name: &str,
    ) -> Result<ManagedSidecarProcess, ServiceManagerError> {
        let config = self.config(name)?.clone();
        if config.mode != ServiceMode::Sidecar {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "service {} is not configured as sidecar",
                config.name
            )));
        }
        config.validate()?;
        let binary = config.binary.clone().expect("validated sidecar binary");
        let mut command = Command::new(binary);
        command.args(&config.args);
        command.envs(config.env.clone());
        command.kill_on_drop(true);
        let child = command
            .spawn()
            .map_err(|err| ServiceManagerError::SpawnFailed {
                service: config.name.clone(),
                source: err,
            })?;

        let mut process = ManagedSidecarProcess { config, child };
        if let Err(err) = process.wait_until_healthy(&self.client).await {
            if let Err(kill_source) = process.child.kill().await {
                return Err(ServiceManagerError::StartupCleanupFailed {
                    service: process.config.name.clone(),
                    startup: Box::new(err),
                    kill_source,
                });
            }
            return Err(err);
        }
        Ok(process)
    }

    async fn http_health(
        &self,
        config: &ManagedServiceConfig,
    ) -> Result<ServiceHealth, ServiceManagerError> {
        http_health(&self.client, config).await
    }
}
