//! LLM model context and output capability columns.

use sea_orm::{DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N LLM model capabilities migration only supports PostgreSQL".to_string(),
            ));
        }
        let db = manager.get_connection();
        db.execute(Statement::from_string(
            DbBackend::Postgres,
            "ALTER TABLE llm_config ADD COLUMN IF NOT EXISTS max_context_tokens INTEGER NOT NULL DEFAULT 128000".to_string(),
        ))
        .await?;
        db.execute(Statement::from_string(
            DbBackend::Postgres,
            "ALTER TABLE llm_config ADD COLUMN IF NOT EXISTS max_output_tokens INTEGER NOT NULL DEFAULT 8192".to_string(),
        ))
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N LLM model capabilities migration only supports PostgreSQL".to_string(),
            ));
        }
        let db = manager.get_connection();
        db.execute(Statement::from_string(
            DbBackend::Postgres,
            "ALTER TABLE llm_config DROP COLUMN IF EXISTS max_output_tokens".to_string(),
        ))
        .await?;
        db.execute(Statement::from_string(
            DbBackend::Postgres,
            "ALTER TABLE llm_config DROP COLUMN IF EXISTS max_context_tokens".to_string(),
        ))
        .await?;
        Ok(())
    }
}
