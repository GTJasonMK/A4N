use crate::OrchestratorContext;
use a4n_core::{error::AppError, models::source_sync::source_sync_run};

impl OrchestratorContext {
    pub async fn source_sync_run_by_id(
        &self,
        sync_run_id: i64,
    ) -> Result<Option<source_sync_run::Model>, AppError> {
        a4n_db::source_sync::source_sync_run_by_id(self.db.connection(), sync_run_id)
            .await
            .map_err(AppError::Database)
    }
}
