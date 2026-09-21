//! Durable model-call cost ledger.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS model_usage_record (
                    id BIGSERIAL PRIMARY KEY,
                    request_id TEXT NOT NULL UNIQUE,
                    user_id BIGINT,
                    project_id BIGINT,
                    root_project_id BIGINT,
                    workflow_run_id BIGINT,
                    generation_run_id BIGINT,
                    workflow TEXT NOT NULL,
                    step_key TEXT NOT NULL,
                    call_key TEXT,
                    source TEXT,
                    capability TEXT NOT NULL,
                    provider_config_id BIGINT,
                    provider TEXT,
                    model TEXT,
                    status TEXT NOT NULL,
                    confidence TEXT NOT NULL,
                    currency TEXT NOT NULL,
                    amount_micros BIGINT,
                    meter_json JSONB NOT NULL DEFAULT '{}'::jsonb,
                    pricing_snapshot_json JSONB NOT NULL DEFAULT '{}'::jsonb,
                    error_kind TEXT,
                    error_message TEXT,
                    started_at TIMESTAMP NOT NULL,
                    completed_at TIMESTAMP,
                    created_at TIMESTAMP NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_model_usage_record_project_started
                    ON model_usage_record (project_id, started_at DESC);
                CREATE INDEX IF NOT EXISTS idx_model_usage_record_user_started
                    ON model_usage_record (user_id, started_at DESC);
                CREATE INDEX IF NOT EXISTS idx_model_usage_record_capability_started
                    ON model_usage_record (capability, started_at DESC);
                CREATE INDEX IF NOT EXISTS idx_model_usage_record_status
                    ON model_usage_record (status);
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP INDEX IF EXISTS idx_model_usage_record_status;
                DROP INDEX IF EXISTS idx_model_usage_record_capability_started;
                DROP INDEX IF EXISTS idx_model_usage_record_user_started;
                DROP INDEX IF EXISTS idx_model_usage_record_project_started;
                DROP TABLE IF EXISTS model_usage_record;
                "#,
            )
            .await?;
        Ok(())
    }
}
