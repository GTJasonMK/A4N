//! Bind an optimization turn to the durable Agent turn that produced it.

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Optimization Agent turn idempotency migration only supports PostgreSQL".into(),
            ));
        }
        let connection = manager.get_connection();
        connection
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "ALTER TABLE optimization_turn ADD COLUMN IF NOT EXISTS agent_turn_id BIGINT",
            ))
            .await?;
        connection
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "CREATE UNIQUE INDEX IF NOT EXISTS uidx_optimization_turn_agent_turn ON optimization_turn (agent_turn_id) WHERE agent_turn_id IS NOT NULL",
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Optimization Agent turn idempotency migration only supports PostgreSQL".into(),
            ));
        }
        let connection = manager.get_connection();
        connection
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "DROP INDEX IF EXISTS uidx_optimization_turn_agent_turn",
            ))
            .await?;
        connection
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "ALTER TABLE optimization_turn DROP COLUMN IF EXISTS agent_turn_id",
            ))
            .await?;
        Ok(())
    }
}
