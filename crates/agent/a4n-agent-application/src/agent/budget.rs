use super::*;

impl AgentApplication {
    pub async fn run_budget_snapshot(
        &self,
        run_id: i64,
    ) -> Result<agent::AgentBudgetSnapshot, AppError> {
        agent::run_budget_snapshot(self.executor(), run_id)
            .await
            .map_err(agent_db_error_to_app)
    }
}
