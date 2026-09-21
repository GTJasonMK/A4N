//! Persist the explicit protocol used by interactive Agent model calls.

use sea_orm::{DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "LLM Agent protocol migration only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "ALTER TABLE llm_config ADD COLUMN IF NOT EXISTS agent_protocol TEXT NOT NULL DEFAULT 'native_tool_calling'",
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "LLM Agent protocol migration only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "ALTER TABLE llm_config DROP COLUMN IF EXISTS agent_protocol",
            ))
            .await?;
        Ok(())
    }
}
