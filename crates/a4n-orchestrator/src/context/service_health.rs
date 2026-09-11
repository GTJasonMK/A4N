use a4n_error::AppError;

use super::OrchestratorContext;
use crate::service_health::{contracts::ManagedServiceHealthSummary, service_health_summary};

impl OrchestratorContext {
    pub async fn managed_service_health_summaries(
        &self,
    ) -> Result<Vec<ManagedServiceHealthSummary>, AppError> {
        let results = futures::future::join_all(self.service_manager.configs().into_iter().map(
            |config| async move {
                let (health, managed_status) = futures::join!(
                    self.service_manager.health(&config.name),
                    self.service_manager.process_status(&config.name),
                );
                let managed_status = managed_status
                    .map_err(|error| AppError::Internal(anyhow::anyhow!(error.to_string())))?;
                Ok::<_, AppError>(service_health_summary(
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

#[async_trait::async_trait]
impl crate::service_health::ServiceHealthApplicationPort for OrchestratorContext {
    async fn summaries(
        &self,
    ) -> Result<Vec<crate::service_health::ManagedServiceHealthSummary>, a4n_error::AppError> {
        self.managed_service_health_summaries().await
    }
}
