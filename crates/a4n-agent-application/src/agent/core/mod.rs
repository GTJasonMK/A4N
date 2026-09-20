//! Agent application API 与持久化核心。

use super::*;

impl AgentApplication {
    /// Lifecycle shutdown uses the same durable run-tree cancellation owner as
    /// the user-facing control path, but does not impersonate a user.
    pub async fn cancel_run_for_package_shutdown(&self, run_id: i64) -> Result<(), AppError> {
        agent::cancel_run_tree(
            self.connection(),
            run_id,
            Some("功能包停用，取消 Agent run"),
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }

    pub fn new(database: DbHandle) -> Self {
        Self { database }
    }

    pub(super) fn connection(&self) -> &sea_orm::DatabaseConnection {
        self.database.connection()
    }

    pub(super) fn reconcile_canonical_thread(
        &self,
        existing: thread::Model,
        capability: &str,
        expected_kind: &str,
        requested_scope: &Value,
        operation: &str,
    ) -> Result<thread::Model, AppError> {
        if existing.thread_kind != expected_kind || existing.profile_id != capability {
            return Err(AppError::InvalidState {
                current: format!("canonical Agent thread {} 配置不一致", existing.id),
                operation: operation.into(),
            });
        }
        let existing_scope: AgentScope =
            from_json(existing.scope_json.clone(), "agent_thread.scope_json")?;
        let mut requested: AgentScope = from_json(requested_scope.clone(), "requested_scope")?;
        inherit_subagent_allowlists(&existing_scope, &mut requested)?;
        if existing_scope == requested {
            return Ok(existing);
        }
        Err(AppError::InvalidArgument(
            "canonical Agent thread 已有不同 scope；请通过 update_agent_thread 显式修改，不能在重用时覆盖权限".into(),
        ))
    }
}

mod access;
mod interactions;
mod threads;
