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

    pub(super) async fn reconcile_canonical_thread(
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
        let mut existing_scope: AgentScope =
            from_json(existing.scope_json.clone(), "agent_thread.scope_json")?;
        let mut requested: AgentScope = from_json(requested_scope.clone(), "requested_scope")?;
        complete_canonical_resource(&mut existing_scope, &existing)?;
        complete_canonical_resource(&mut requested, &existing)?;
        inherit_subagent_allowlists(&existing_scope, &mut requested)?;
        if !agent_scopes_match_except_chapter_number(&existing_scope, &requested) {
            return Err(AppError::InvalidArgument(
                "canonical Agent thread 已有不同 scope；请通过 update_agent_thread 显式修改，不能在重用时覆盖权限".into(),
            ));
        }
        if serde_json::to_value(&requested)? == existing.scope_json {
            return Ok(existing);
        }

        let tx = self.connection().begin().await?;
        let existing = thread::Entity::find_by_id(existing.id)
            .lock_exclusive()
            .one(&tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Agent thread 不存在".into()))?;
        if existing.thread_kind != expected_kind || existing.profile_id != capability {
            return Err(AppError::InvalidState {
                current: format!("canonical Agent thread {} 配置不一致", existing.id),
                operation: operation.into(),
            });
        }
        let mut existing_scope: AgentScope =
            from_json(existing.scope_json.clone(), "agent_thread.scope_json")?;
        let mut requested: AgentScope = from_json(requested_scope.clone(), "requested_scope")?;
        complete_canonical_resource(&mut existing_scope, &existing)?;
        complete_canonical_resource(&mut requested, &existing)?;
        inherit_subagent_allowlists(&existing_scope, &mut requested)?;
        if !agent_scopes_match_except_chapter_number(&existing_scope, &requested) {
            return Err(AppError::InvalidArgument(
                "canonical Agent thread 已有不同 scope；请通过 update_agent_thread 显式修改，不能在重用时覆盖权限".into(),
            ));
        }
        if serde_json::to_value(&requested)? == existing.scope_json {
            tx.commit().await?;
            return Ok(existing);
        }

        existing_scope.chapter_number = requested.chapter_number;
        let mut active: thread::ActiveModel = existing.into();
        active.scope_json = Set(serde_json::to_value(existing_scope)?);
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        let updated = active.update(&tx).await.map_err(AppError::Database)?;
        tx.commit().await?;
        Ok(updated)
    }
}

fn complete_canonical_resource(
    scope: &mut AgentScope,
    thread: &thread::Model,
) -> Result<(), AppError> {
    match (
        thread.resource_type.as_deref(),
        thread.resource_id.as_deref(),
    ) {
        (Some(kind), Some(id)) => complete_scope_resource(scope, kind, id),
        (None, None) => Ok(()),
        _ => Err(AppError::InvalidArgument(
            "canonical Agent 资源身份不完整".into(),
        )),
    }
}

mod access;
mod interactions;
mod lineage;
mod threads;
