use super::config::ManagedServiceConfig;
use super::errors::ServiceManagerError;
use super::health::http_health;
use a4n_service_protocol::HealthStatus;
use std::time::Duration;
use tokio::process::Child;

#[derive(Debug)]
pub struct ManagedSidecarProcess {
    pub(super) config: ManagedServiceConfig,
    pub(super) child: Child,
}

impl ManagedSidecarProcess {
    pub fn service_name(&self) -> &str {
        &self.config.name
    }

    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    pub async fn stop(mut self) -> Result<(), ServiceManagerError> {
        self.child
            .kill()
            .await
            .map_err(|err| ServiceManagerError::StopFailed {
                service: self.config.name.clone(),
                source: err,
            })
    }

    pub(super) async fn wait_until_healthy(
        &mut self,
        client: &reqwest::Client,
    ) -> Result<(), ServiceManagerError> {
        let deadline = Duration::from_millis(self.config.startup_timeout_ms);
        let start = std::time::Instant::now();
        let mut last_error = None;

        while start.elapsed() < deadline {
            if let Some(status) =
                self.child
                    .try_wait()
                    .map_err(|err| ServiceManagerError::ProcessStatusFailed {
                        service: self.config.name.clone(),
                        source: err,
                    })?
            {
                return Err(ServiceManagerError::Exited {
                    service: self.config.name.clone(),
                    status: status.code(),
                });
            }
            match http_health(client, &self.config).await {
                Ok(health) if health.status == HealthStatus::Healthy => return Ok(()),
                Ok(health) => {
                    last_error = Some(ServiceManagerError::Unhealthy {
                        service: self.config.name.clone(),
                        status: health.status,
                    });
                }
                Err(err) => last_error = Some(err),
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(last_error.unwrap_or_else(|| ServiceManagerError::Timeout {
            service: self.config.name.clone(),
            timeout_ms: self.config.startup_timeout_ms,
        }))
    }
}

impl Drop for ManagedSidecarProcess {
    fn drop(&mut self) {
        if let Err(error) = self.child.start_kill() {
            tracing::warn!(
                service = %self.config.name,
                %error,
                "释放 managed sidecar 时发送终止信号失败"
            );
        }
    }
}
