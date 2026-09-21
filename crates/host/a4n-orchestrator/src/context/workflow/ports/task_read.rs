//! task read port。

use super::*;

#[async_trait::async_trait]
impl a4n_api_contract::task::TaskReadApplicationPort for OrchestratorContext {
    async fn history(
        &self,
        project_id: Option<i64>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<a4n_api_contract::task::TaskInfoSummary>, AppError> {
        self.workflow.task_history(project_id, offset, limit).await
    }
}
