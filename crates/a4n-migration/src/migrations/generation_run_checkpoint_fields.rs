//! Backfill generation run checkpoint columns for databases that already ran older workflow migrations.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N generation run checkpoint fields migration only supports PostgreSQL"
                    .to_string(),
            ));
        }

        execute_pg_sql(
            manager,
            "ALTER TABLE generation_run ADD COLUMN IF NOT EXISTS workflow_run_id BIGINT",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE generation_run ADD COLUMN IF NOT EXISTS call_key TEXT",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE generation_run ADD COLUMN IF NOT EXISTS input_fingerprint_sha256 TEXT",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE generation_run ADD COLUMN IF NOT EXISTS input_fingerprint_json JSONB",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE generation_run ADD COLUMN IF NOT EXISTS raw_response_json JSONB",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE generation_run ADD COLUMN IF NOT EXISTS parsed_output_json JSONB",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE generation_run ADD COLUMN IF NOT EXISTS validation_json JSONB",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE generation_run ADD COLUMN IF NOT EXISTS artifact_refs_json JSONB",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE generation_run ADD COLUMN IF NOT EXISTS attempt INTEGER NOT NULL DEFAULT 1",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE task_queue ADD COLUMN IF NOT EXISTS workflow_run_id BIGINT",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE workflow_checkpoint ADD COLUMN IF NOT EXISTS input_fingerprint_sha256 TEXT",
        )
        .await?;

        for sql in [
            "CREATE INDEX IF NOT EXISTS idx_workflow_checkpoint_run_step_fingerprint ON workflow_checkpoint (workflow_run_id, step_key, input_fingerprint_sha256)",
            "CREATE INDEX IF NOT EXISTS idx_generation_run_workflow_run ON generation_run (workflow_run_id)",
            "CREATE INDEX IF NOT EXISTS idx_generation_run_workflow_step_call ON generation_run (workflow_run_id, step_key, call_key)",
            "CREATE INDEX IF NOT EXISTS idx_generation_run_workflow_call_fingerprint ON generation_run (workflow_run_id, step_key, call_key, input_fingerprint_sha256)",
            "CREATE INDEX IF NOT EXISTS idx_generation_run_workflow_status ON generation_run (workflow_run_id, status)",
            "CREATE INDEX IF NOT EXISTS idx_task_queue_workflow_run ON task_queue (workflow_run_id)",
        ] {
            execute_pg_sql(manager, sql).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N generation run checkpoint fields migration only supports PostgreSQL"
                    .to_string(),
            ));
        }

        for sql in [
            "DROP INDEX IF EXISTS idx_workflow_checkpoint_run_step_fingerprint",
            "DROP INDEX IF EXISTS idx_task_queue_workflow_run",
            "DROP INDEX IF EXISTS idx_generation_run_workflow_status",
            "DROP INDEX IF EXISTS idx_generation_run_workflow_call_fingerprint",
            "DROP INDEX IF EXISTS idx_generation_run_workflow_step_call",
            "DROP INDEX IF EXISTS idx_generation_run_workflow_run",
            r#"
            ALTER TABLE generation_run
                DROP COLUMN IF EXISTS workflow_run_id,
                DROP COLUMN IF EXISTS call_key,
                DROP COLUMN IF EXISTS input_fingerprint_sha256,
                DROP COLUMN IF EXISTS input_fingerprint_json,
                DROP COLUMN IF EXISTS raw_response_json,
                DROP COLUMN IF EXISTS parsed_output_json,
                DROP COLUMN IF EXISTS validation_json,
                DROP COLUMN IF EXISTS artifact_refs_json,
                DROP COLUMN IF EXISTS attempt
            "#,
            "ALTER TABLE task_queue DROP COLUMN IF EXISTS workflow_run_id",
            "ALTER TABLE workflow_checkpoint DROP COLUMN IF EXISTS input_fingerprint_sha256",
        ] {
            execute_pg_sql(manager, sql).await?;
        }

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
