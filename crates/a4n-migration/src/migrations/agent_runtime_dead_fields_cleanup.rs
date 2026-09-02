//! Remove columns from the first Agent runtime prototype that are no longer
//! part of the durable contract.
//!
//! The current schema keeps one source of truth: Turn owns per-turn execution
//! settings, Run owns lifecycle/usage, and Execution stores arguments/results.
//! These legacy mirrors were never read by the current runtime and must not
//! survive as a second state surface on deployed databases.

use sea_orm::{ConnectionTrait, DbBackend, DbErr};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "agent_runtime_dead_fields_cleanup";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N Agent dead-field cleanup migration only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE agent_thread
                    DROP COLUMN IF EXISTS model_override_json,
                    DROP COLUMN IF EXISTS execution_mode;
                 ALTER TABLE agent_message
                    DROP COLUMN IF EXISTS completed_at;
                 ALTER TABLE agent_turn
                    DROP COLUMN IF EXISTS current_attempt;
                 ALTER TABLE agent_execution
                    DROP COLUMN IF EXISTS task_id,
                    DROP COLUMN IF EXISTS workflow_run_id,
                    DROP COLUMN IF EXISTS generation_task_id,
                    DROP COLUMN IF EXISTS task_ids,
                    DROP COLUMN IF EXISTS workflow_run_ids,
                    DROP COLUMN IF EXISTS generation_task_ids;",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(format!(
            "{MIGRATION_NAME} 不可逆：旧 Agent 镜像字段已由规范持久状态取代"
        )))
    }
}
