use a4n_core::error::AppError;

use super::OrchestratorContext;
use crate::service_health::{contracts::ManagedServiceHealthSummary, service_health_summary};

impl OrchestratorContext {
    pub async fn managed_service_health_summaries(
        &self,
    ) -> Result<Vec<ManagedServiceHealthSummary>, AppError> {
        let results = futures::future::join_all(self.service_manager.configs().into_iter().map(
            |config| async move {
                let health = self.service_manager.health(&config.name).await;
                let managed_status = self
                    .service_manager
                    .process_status(&config.name)
                    .await
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
