use std::collections::HashMap;

use a4n_core::error::AppError;

use super::{ManagedServicePidMap, OrchestratorContext};
use crate::service_health::{ManagedServiceHealthSummary, service_health_summary};

impl OrchestratorContext {
    pub fn with_managed_service_pids(mut self, managed_service_pids: ManagedServicePidMap) -> Self {
        self.managed_service_pids = Some(managed_service_pids);
        self
    }

    pub async fn managed_service_health_summaries(
        &self,
    ) -> Result<Vec<ManagedServiceHealthSummary>, AppError> {
        let mut summaries = Vec::new();
        let managed_pids = managed_service_pid_snapshot(self.managed_service_pids.as_ref())?;
        for config in self.service_manager.configs() {
            let health = self.service_manager.health(&config.name).await;
            let managed_pid = managed_pids.get(&config.name).copied();
            summaries.push(service_health_summary(
                config,
                health,
                self.service_pid_dir.as_deref(),
                managed_pid,
            ));
        }
        Ok(summaries)
    }
}

fn managed_service_pid_snapshot(
    pids: Option<&ManagedServicePidMap>,
) -> Result<HashMap<String, u32>, AppError> {
    let Some(pids) = pids else {
        return Ok(HashMap::new());
    };
    pids.read()
        .map(|pids| pids.clone())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("受管 sidecar PID 锁已中毒")))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::{Arc, RwLock};

    use super::managed_service_pid_snapshot;

    #[test]
    fn poisoned_managed_service_pid_lock_is_reported() {
        let pids = Arc::new(RwLock::new(HashMap::new()));
        let poison_result = catch_unwind(AssertUnwindSafe({
            let pids = Arc::clone(&pids);
            move || {
                let _guard = pids.write().expect("PID lock should start healthy");
                panic!("poison PID lock for error-path coverage");
            }
        }));
        assert!(poison_result.is_err());

        let error = managed_service_pid_snapshot(Some(&pids))
            .expect_err("poisoned PID lock must fail explicitly");
        assert!(error.to_string().contains("受管 sidecar PID 锁已中毒"));
    }
}
