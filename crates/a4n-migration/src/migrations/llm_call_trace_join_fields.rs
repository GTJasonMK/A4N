//! Add stable join keys to LLM call traces.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in [
            "ALTER TABLE llm_call_trace ADD COLUMN IF NOT EXISTS workflow_run_id BIGINT",
            "ALTER TABLE llm_call_trace ADD COLUMN IF NOT EXISTS call_key TEXT",
            "ALTER TABLE llm_call_trace ADD COLUMN IF NOT EXISTS input_fingerprint_sha256 TEXT",
            "CREATE INDEX IF NOT EXISTS idx_llm_call_trace_workflow_run ON llm_call_trace (workflow_run_id)",
            "CREATE INDEX IF NOT EXISTS idx_llm_call_trace_workflow_step_call ON llm_call_trace (workflow_run_id, step_key, call_key)",
            "CREATE INDEX IF NOT EXISTS idx_llm_call_trace_workflow_call_fingerprint ON llm_call_trace (workflow_run_id, step_key, call_key, input_fingerprint_sha256)",
        ] {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in [
            "DROP INDEX IF EXISTS idx_llm_call_trace_workflow_call_fingerprint",
            "DROP INDEX IF EXISTS idx_llm_call_trace_workflow_step_call",
            "DROP INDEX IF EXISTS idx_llm_call_trace_workflow_run",
            r#"
            ALTER TABLE llm_call_trace
                DROP COLUMN IF EXISTS input_fingerprint_sha256,
                DROP COLUMN IF EXISTS call_key,
                DROP COLUMN IF EXISTS workflow_run_id
            "#,
        ] {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
