//! Remove the unused Agent Run checkpoint mirror.
//!
//! Agent recovery replays durable executions, messages and inbox records.  The
//! old step-only checkpoint was never read by a worker, so retaining it created
//! a second, misleading source of progress state.

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "agent_runtime_checkpoint_cleanup";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N Agent checkpoint cleanup migration only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "ALTER TABLE agent_run
                    DROP COLUMN IF EXISTS checkpoint_json,
                    DROP COLUMN IF EXISTS checkpoint_revision"
                    .to_string(),
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(format!(
            "{MIGRATION_NAME} 不可逆：Agent Run checkpoint 是未消费的重复状态"
        )))
    }
}
