//! Retire pending intents for the removed project-wide media reset worker.

use sea_orm::{ConnectionTrait, DbBackend, DbErr};
use sea_orm_migration::prelude::*;

const RETIRE_SQL: &str = r#"
UPDATE project_cleanup_outbox
SET status = 'completed',
    outcome = 'completed',
    lease_token = NULL,
    lease_expires_at = NULL,
    completed_at = COALESCE(completed_at, NOW()),
    updated_at = NOW(),
    last_error = COALESCE(last_error, '媒体工作流 v2 整项目 reset 已废弃')
WHERE cleanup_kind = 'media_workflow_contract_v2_reset'
  AND status IN ('pending', 'running');
"#;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "媒体工作流 reset 退役迁移仅支持 PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(RETIRE_SQL)
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "media_workflow_reset_retirement 不可逆：已退役的 reset intent 不会恢复".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::RETIRE_SQL;

    #[test]
    fn retires_only_pending_removed_media_reset_intents() {
        assert!(RETIRE_SQL.contains("media_workflow_contract_v2_reset"));
        assert!(RETIRE_SQL.contains("status IN ('pending', 'running')"));
        assert!(RETIRE_SQL.contains("outcome = 'completed'"));
    }
}
