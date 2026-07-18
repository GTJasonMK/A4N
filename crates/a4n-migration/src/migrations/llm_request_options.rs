//! LLM provider-specific request options.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N LLM request options migration only supports PostgreSQL".to_string(),
            ));
        }

        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "ALTER TABLE llm_config ADD COLUMN IF NOT EXISTS request_options_json JSONB"
                    .to_string(),
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N LLM request options migration only supports PostgreSQL".to_string(),
            ));
        }

        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "ALTER TABLE llm_config DROP COLUMN IF EXISTS request_options_json".to_string(),
            ))
            .await?;
        Ok(())
    }
}
