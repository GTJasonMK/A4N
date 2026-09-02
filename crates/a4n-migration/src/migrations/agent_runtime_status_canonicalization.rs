//! Canonicalize status values left by the first Agent runtime prototype.
//!
//! The current contracts use `complete`/`interrupted` for messages and
//! `active` for Turns.  Inbox `claimed` is intentionally not normalized: it is
//! the live inbox lease state, not a historical alias.

use sea_orm::{ConnectionTrait, DbBackend, DbErr};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "agent_runtime_status_canonicalization";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N Agent status canonicalization migration only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(
                "UPDATE agent_message
                    SET status = CASE status
                        WHEN 'completed' THEN 'complete'
                        WHEN 'failed' THEN 'interrupted'
                        WHEN 'cancelled' THEN 'interrupted'
                        ELSE status
                    END
                  WHERE status IN ('completed', 'failed', 'cancelled');
                  UPDATE agent_turn
                    SET status = 'active'
                  WHERE status = 'running'",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(format!(
            "{MIGRATION_NAME} 不可逆：历史状态与规范状态无法可靠区分"
        )))
    }
}
