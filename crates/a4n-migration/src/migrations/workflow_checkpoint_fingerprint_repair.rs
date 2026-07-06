//! Repair workflow checkpoint fingerprint columns for databases that already applied older migrations.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N workflow checkpoint fingerprint repair migration only supports PostgreSQL"
                    .to_string(),
            ));
        }

        for sql in [
            "ALTER TABLE workflow_checkpoint ADD COLUMN IF NOT EXISTS input_fingerprint_sha256 TEXT",
            "CREATE INDEX IF NOT EXISTS idx_workflow_checkpoint_run_step_fingerprint ON workflow_checkpoint (workflow_run_id, step_key, input_fingerprint_sha256)",
        ] {
            execute_pg_sql(manager, sql).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N workflow checkpoint fingerprint repair migration only supports PostgreSQL"
                    .to_string(),
            ));
        }

        execute_pg_sql(
            manager,
            "DROP INDEX IF EXISTS idx_workflow_checkpoint_run_step_fingerprint",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE workflow_checkpoint DROP COLUMN IF EXISTS input_fingerprint_sha256",
        )
        .await?;

        Ok(())
    }
}

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}
