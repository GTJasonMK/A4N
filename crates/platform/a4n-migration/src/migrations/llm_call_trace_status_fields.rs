//! Backfill status and error fields for LLM call traces.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N LLM call trace status fields migration only supports PostgreSQL".to_string(),
            ));
        }

        for sql in [
            "ALTER TABLE llm_call_trace ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'completed'",
            "ALTER TABLE llm_call_trace ADD COLUMN IF NOT EXISTS error_kind TEXT",
            "ALTER TABLE llm_call_trace ADD COLUMN IF NOT EXISTS error_message TEXT",
            "ALTER TABLE llm_call_trace ADD COLUMN IF NOT EXISTS finish_reason TEXT",
            "CREATE INDEX IF NOT EXISTS idx_llm_call_trace_status ON llm_call_trace (status)",
        ] {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N LLM call trace status fields migration only supports PostgreSQL".to_string(),
            ));
        }

        for sql in [
            "DROP INDEX IF EXISTS idx_llm_call_trace_status",
            r#"
            ALTER TABLE llm_call_trace
                DROP COLUMN IF EXISTS finish_reason,
                DROP COLUMN IF EXISTS error_message,
                DROP COLUMN IF EXISTS error_kind,
                DROP COLUMN IF EXISTS status
            "#,
        ] {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }
}

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(
            manager.get_database_backend(),
            sql.to_string(),
        ))
        .await?;
    Ok(())
}
