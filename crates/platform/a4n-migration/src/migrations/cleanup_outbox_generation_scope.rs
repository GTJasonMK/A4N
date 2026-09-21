//! 将既有唯一 cleanup outbox 扩展为 generation 可用的 typed scope。

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;
use sha2::{Digest, Sha256};

const ADD_COLUMNS_SQL: &str = r#"
ALTER TABLE project_cleanup_outbox
    ALTER COLUMN project_id DROP NOT NULL,
    ADD COLUMN IF NOT EXISTS requested_by_user_id BIGINT,
    ADD COLUMN IF NOT EXISTS task_id TEXT,
    ADD COLUMN IF NOT EXISTS attempt INTEGER,
    ADD COLUMN IF NOT EXISTS payload_version INTEGER,
    ADD COLUMN IF NOT EXISTS payload_sha256 TEXT,
    ADD COLUMN IF NOT EXISTS observation_count INTEGER DEFAULT 0,
    ADD COLUMN IF NOT EXISTS outcome TEXT;
"#;

const FINALIZE_CONTRACT_SQL: &str = r#"
ALTER TABLE project_cleanup_outbox
    ALTER COLUMN payload_version SET NOT NULL,
    ALTER COLUMN payload_sha256 SET NOT NULL,
    ALTER COLUMN observation_count SET DEFAULT 0,
    ALTER COLUMN observation_count SET NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_cleanup_outbox_payload_contract'
    ) THEN
        ALTER TABLE project_cleanup_outbox
            ADD CONSTRAINT chk_cleanup_outbox_payload_contract CHECK (
                payload_version > 0
                AND payload_sha256 ~ '^[0-9a-f]{64}$'
            );
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_cleanup_outbox_generation_scope'
    ) THEN
        ALTER TABLE project_cleanup_outbox
            ADD CONSTRAINT chk_cleanup_outbox_generation_scope CHECK (
                (
                    cleanup_kind IN (
                        'generation_provider_job_cleanup',
                        'generation_staging_artifact_delete',
                        'generation_artifact_unbind'
                    )
                    AND requested_by_user_id BETWEEN 1 AND 9007199254740991
                    AND task_id IS NOT NULL
                    AND attempt > 0
                    AND (project_id IS NULL OR project_id BETWEEN 1 AND 9007199254740991)
                )
                OR (
                    cleanup_kind NOT IN (
                        'generation_provider_job_cleanup',
                        'generation_staging_artifact_delete',
                        'generation_artifact_unbind'
                    )
                    AND requested_by_user_id IS NULL
                    AND task_id IS NULL
                    AND attempt IS NULL
                    AND project_id BETWEEN 1 AND 9007199254740991
                )
            );
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_cleanup_outbox_counters_outcome'
    ) THEN
        ALTER TABLE project_cleanup_outbox
            ADD CONSTRAINT chk_cleanup_outbox_counters_outcome CHECK (
                attempt_count >= 0
                AND observation_count >= 0
                AND (
                    (status IN ('pending', 'running') AND outcome IS NULL)
                    OR status = 'failed'
                    OR (
                        status = 'completed'
                        AND (
                            (
                                cleanup_kind IN (
                                    'generation_provider_job_cleanup',
                                    'generation_staging_artifact_delete',
                                    'generation_artifact_unbind'
                                )
                                AND outcome IN (
                                    'cancelled',
                                    'deleted',
                                    'unbound',
                                    'not_found',
                                    'unsupported',
                                    'remote_failed',
                                    'delete_scheduled'
                                )
                            )
                            OR (
                                cleanup_kind NOT IN (
                                    'generation_provider_job_cleanup',
                                    'generation_staging_artifact_delete',
                                    'generation_artifact_unbind'
                                )
                                AND outcome = 'completed'
                            )
                        )
                    )
                )
            );
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_cleanup_outbox_generation_task
    ON project_cleanup_outbox (task_id, attempt, status)
    WHERE task_id IS NOT NULL;
"#;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "cleanup outbox generation scope 仅支持 PostgreSQL".to_string(),
            ));
        }
        let db = manager.get_connection();
        db.execute_unprepared(ADD_COLUMNS_SQL).await?;
        let rows = db
            .query_all(Statement::from_string(
                DbBackend::Postgres,
                "SELECT id, status, payload_json FROM project_cleanup_outbox ORDER BY id",
            ))
            .await?;
        for row in rows {
            let id: i64 = row.try_get("", "id")?;
            let status: String = row.try_get("", "status")?;
            let payload: serde_json::Value = row.try_get("", "payload_json")?;
            let version = payload
                .get("version")
                .and_then(serde_json::Value::as_i64)
                .and_then(|value| <i32 as TryFrom<i64>>::try_from(value).ok())
                .filter(|value| *value > 0)
                .ok_or_else(|| {
                    DbErr::Custom(format!("cleanup outbox {id} payload version 无效"))
                })?;
            let canonical = serde_json_canonicalizer::to_vec(&payload).map_err(|error| {
                DbErr::Custom(format!("cleanup outbox {id} canonical JSON 失败: {error}"))
            })?;
            let hash = format!("{:x}", Sha256::digest(canonical));
            let outcome = (status == "completed").then_some("completed".to_string());
            db.execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"
UPDATE project_cleanup_outbox
SET payload_version = $2,
    payload_sha256 = $3,
    observation_count = COALESCE(observation_count, 0),
    outcome = COALESCE(outcome, $4)
WHERE id = $1
"#,
                [id.into(), version.into(), hash.into(), outcome.into()],
            ))
            .await?;
        }
        db.execute_unprepared(FINALIZE_CONTRACT_SQL).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "cleanup_outbox_generation_scope 是前向迁移，不能安全回滚".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_keeps_one_outbox_and_adds_generation_scope() {
        assert!(ADD_COLUMNS_SQL.contains("ALTER TABLE project_cleanup_outbox"));
        assert!(FINALIZE_CONTRACT_SQL.contains("generation_provider_job_cleanup"));
        assert!(FINALIZE_CONTRACT_SQL.contains("payload_sha256 ~ '^[0-9a-f]{64}$'"));
        assert!(!FINALIZE_CONTRACT_SQL.contains("CREATE TABLE"));
    }
}
