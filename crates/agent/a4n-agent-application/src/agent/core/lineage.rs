use super::*;

impl AgentApplication {
    pub(crate) async fn delegation_depth_for_run(
        &self,
        run_id: i64,
        parent_run_id: Option<i64>,
    ) -> Result<u8, AppError> {
        let mut depth = 0_u8;
        let mut ancestor = parent_run_id;
        let mut seen = HashSet::new();
        while let Some(parent_id) = ancestor {
            if !seen.insert(parent_id) {
                return Err(AppError::InvalidState {
                    current: format!("Agent run lineage cycle detected: {run_id}"),
                    operation: "读取 Agent run lineage".into(),
                });
            }
            depth = depth.saturating_add(1);
            ancestor = agent::run_by_id(self.executor(), parent_id)
                .await
                .map_err(agent_db_error_to_app)?
                .and_then(|row| row.parent_run_id);
        }
        Ok(depth)
    }
}
