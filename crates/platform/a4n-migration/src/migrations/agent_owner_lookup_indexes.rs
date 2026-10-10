//! Indexes for high-cardinality Agent owner lookups.

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Agent owner lookup indexes only support PostgreSQL".into(),
            ));
        }
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "CREATE INDEX IF NOT EXISTS idx_agent_execution_generation_owner_lookup
                 ON agent_execution (
                     (resource_preconditions_json->>'generation_task_id'),
                     (resource_preconditions_json->>'generation_attempt'),
                     run_id
                 )",
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Agent owner lookup indexes only support PostgreSQL".into(),
            ));
        }
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "DROP INDEX IF EXISTS idx_agent_execution_generation_owner_lookup",
            ))
            .await?;
        Ok(())
    }
}
