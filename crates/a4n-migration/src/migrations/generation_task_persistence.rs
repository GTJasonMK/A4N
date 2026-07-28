//! 增量生成任务 checkpoint、启动幂等映射和生命周期 gate。

use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

const UP_SQL: &str = r#"
ALTER TABLE "user" ADD COLUMN IF NOT EXISTS lifecycle_gate TEXT;
UPDATE "user"
SET lifecycle_gate = CASE WHEN is_active THEN 'active' ELSE 'disabled' END;
ALTER TABLE "user"
    ALTER COLUMN lifecycle_gate SET DEFAULT 'active',
    ALTER COLUMN lifecycle_gate SET NOT NULL;

ALTER TABLE novel_project
    ADD COLUMN IF NOT EXISTS lifecycle_gate TEXT NOT NULL DEFAULT 'active';
UPDATE novel_project
SET lifecycle_gate = 'active'
WHERE lifecycle_gate IS NULL;
ALTER TABLE novel_project
    ALTER COLUMN lifecycle_gate SET DEFAULT 'active',
    ALTER COLUMN lifecycle_gate SET NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_user_lifecycle_gate'
          AND conrelid = '"user"'::regclass
    ) THEN
        ALTER TABLE "user"
            ADD CONSTRAINT chk_user_lifecycle_gate
            CHECK (lifecycle_gate IN ('active', 'disabling', 'disabled'));
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_novel_project_lifecycle_gate'
          AND conrelid = 'novel_project'::regclass
    ) THEN
        ALTER TABLE novel_project
            ADD CONSTRAINT chk_novel_project_lifecycle_gate
            CHECK (lifecycle_gate IN ('active', 'deleting', 'resetting'));
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_user_lifecycle_gate ON "user" (lifecycle_gate, id);
CREATE INDEX IF NOT EXISTS idx_novel_project_lifecycle_gate
    ON novel_project (lifecycle_gate, id);

CREATE TABLE generation_task_checkpoint (
    task_id TEXT PRIMARY KEY
        REFERENCES task_queue(id) ON DELETE CASCADE,
    business_key TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    requested_by_user_id BIGINT NOT NULL,
    task_kind TEXT NOT NULL,
    delivery_mode TEXT NOT NULL,
    batch_policy TEXT,
    project_id BIGINT,
    root_project_id BIGINT,
    workflow_run_id BIGINT,
    generation_run_id BIGINT,
    target_kind TEXT,
    target_id TEXT,
    target_revision TEXT,
    root_task_id TEXT NOT NULL
        REFERENCES generation_task_checkpoint(task_id) ON DELETE CASCADE,
    parent_task_id TEXT
        REFERENCES generation_task_checkpoint(task_id) ON DELETE CASCADE,
    item_key TEXT,
    retry_of_task_id TEXT
        REFERENCES generation_task_checkpoint(task_id) ON DELETE SET NULL,
    status TEXT NOT NULL,
    checkpoint_revision BIGINT NOT NULL,
    covers_through_sequence BIGINT NOT NULL,
    reserved_through_sequence BIGINT NOT NULL,
    public_snapshot_json JSONB,
    public_snapshot_sha256 TEXT,
    step_key TEXT,
    call_key TEXT,
    provider_kind TEXT,
    provider_model TEXT,
    provider_job_id TEXT,
    provider_idempotency_key_sha256 TEXT,
    resume_metadata_version INTEGER,
    execution_checkpoint_json JSONB,
    execution_checkpoint_sha256 TEXT,
    input_fingerprint_sha256 TEXT NOT NULL,
    result_kind TEXT,
    result_id TEXT,
    result_revision TEXT,
    expires_at TIMESTAMP WITHOUT TIME ZONE,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    completed_at TIMESTAMP WITHOUT TIME ZONE,
    CONSTRAINT chk_generation_task_id_uuid_v4 CHECK (
        length(task_id) = 36
        AND task_id = lower(task_id)
        AND substring(task_id, 15, 1) = '4'
        AND substring(task_id, 20, 1) IN ('8', '9', 'a', 'b')
        AND task_id::uuid IS NOT NULL
    ),
    CONSTRAINT chk_generation_task_business_key CHECK (
        octet_length(business_key) BETWEEN 2 AND 2048
    ),
    CONSTRAINT chk_generation_task_attempt CHECK (attempt > 0),
    CONSTRAINT chk_generation_task_owner_id CHECK (
        requested_by_user_id BETWEEN 1 AND 9007199254740991
    ),
    CONSTRAINT chk_generation_task_kind CHECK (
        octet_length(btrim(task_kind)) BETWEEN 1 AND 128
    ),
    CONSTRAINT chk_generation_task_delivery_mode CHECK (
        delivery_mode IN ('stream', 'stage_only')
    ),
    CONSTRAINT chk_generation_task_batch_policy CHECK (
        batch_policy IS NULL OR batch_policy IN ('atomic', 'best_effort')
    ),
    CONSTRAINT chk_generation_task_project_ids CHECK (
        (project_id IS NULL OR project_id BETWEEN 1 AND 9007199254740991)
        AND (root_project_id IS NULL OR root_project_id BETWEEN 1 AND 9007199254740991)
        AND (workflow_run_id IS NULL OR workflow_run_id BETWEEN 1 AND 9007199254740991)
        AND (generation_run_id IS NULL OR generation_run_id BETWEEN 1 AND 9007199254740991)
    ),
    CONSTRAINT chk_generation_task_target CHECK (
        (target_kind IS NULL AND target_id IS NULL AND target_revision IS NULL)
        OR (
            octet_length(btrim(target_kind)) BETWEEN 1 AND 128
            AND octet_length(btrim(target_id)) BETWEEN 1 AND 256
            AND (target_revision IS NULL OR octet_length(btrim(target_revision)) BETWEEN 1 AND 256)
        )
    ),
    CONSTRAINT chk_generation_task_lineage CHECK (
        (
            root_task_id = task_id
            AND parent_task_id IS NULL
            AND item_key IS NULL
        )
        OR (
            root_task_id <> task_id
            AND parent_task_id IS NOT NULL
            AND octet_length(btrim(item_key)) BETWEEN 1 AND 256
        )
    ),
    CONSTRAINT chk_generation_task_retry CHECK (
        retry_of_task_id IS NULL OR retry_of_task_id <> task_id
    ),
    CONSTRAINT chk_generation_task_status CHECK (
        status IN (
            'reserved', 'running', 'finalizing', 'cancelling',
            'completed', 'failed', 'cancelled', 'interrupted'
        )
    ),
    CONSTRAINT chk_generation_task_sequence CHECK (
        checkpoint_revision BETWEEN 1 AND 9007199254740991
        AND covers_through_sequence BETWEEN 1 AND 9007199254740991
        AND reserved_through_sequence BETWEEN covers_through_sequence AND 9007199254740991
    ),
    CONSTRAINT chk_generation_task_public_snapshot CHECK (
        (public_snapshot_json IS NULL) = (public_snapshot_sha256 IS NULL)
        AND (
            status IN ('completed', 'failed', 'cancelled', 'interrupted')
            OR public_snapshot_json IS NOT NULL
        )
        AND (
            public_snapshot_sha256 IS NULL
            OR public_snapshot_sha256 ~ '^[0-9a-f]{64}$'
        )
    ),
    CONSTRAINT chk_generation_task_execution_checkpoint CHECK (
        (
            execution_checkpoint_json IS NULL
            AND execution_checkpoint_sha256 IS NULL
            AND resume_metadata_version IS NULL
        )
        OR (
            execution_checkpoint_json IS NOT NULL
            AND execution_checkpoint_sha256 ~ '^[0-9a-f]{64}$'
            AND resume_metadata_version > 0
        )
    ),
    CONSTRAINT chk_generation_task_execution_provider CHECK (
        execution_checkpoint_json IS NULL OR provider_kind IS NOT NULL
    ),
    CONSTRAINT chk_generation_task_provider_job CHECK (
        provider_job_id IS NULL
        OR (
            provider_kind IS NOT NULL
            AND provider_idempotency_key_sha256 IS NOT NULL
        )
    ),
    CONSTRAINT chk_generation_task_provider_idempotency CHECK (
        provider_idempotency_key_sha256 IS NULL
        OR provider_idempotency_key_sha256 ~ '^[0-9a-f]{64}$'
    ),
    CONSTRAINT chk_generation_task_input_fingerprint CHECK (
        input_fingerprint_sha256 ~ '^[0-9a-f]{64}$'
    ),
    CONSTRAINT chk_generation_task_result CHECK (
        (result_kind IS NULL AND result_id IS NULL AND result_revision IS NULL)
        OR (
            octet_length(btrim(result_kind)) BETWEEN 1 AND 128
            AND octet_length(btrim(result_id)) BETWEEN 1 AND 256
            AND octet_length(btrim(result_revision)) BETWEEN 1 AND 256
        )
    ),
    CONSTRAINT chk_generation_task_timestamps CHECK (
        updated_at >= created_at
        AND (completed_at IS NULL OR completed_at >= created_at)
        AND (expires_at IS NULL OR expires_at >= updated_at)
    )
);

CREATE UNIQUE INDEX uq_generation_task_business_attempt
    ON generation_task_checkpoint (business_key, attempt);
CREATE UNIQUE INDEX uq_generation_task_active_business
    ON generation_task_checkpoint (business_key)
    WHERE status IN ('reserved', 'running', 'finalizing', 'cancelling');
CREATE INDEX idx_generation_task_owner_status_updated
    ON generation_task_checkpoint (requested_by_user_id, status, updated_at DESC);
CREATE INDEX idx_generation_task_project_status_updated
    ON generation_task_checkpoint (project_id, status, updated_at DESC)
    WHERE project_id IS NOT NULL;
CREATE INDEX idx_generation_task_root_project_status_updated
    ON generation_task_checkpoint (root_project_id, status, updated_at DESC)
    WHERE root_project_id IS NOT NULL;
CREATE INDEX idx_generation_task_status_expires
    ON generation_task_checkpoint (status, expires_at);
CREATE INDEX idx_generation_task_parent
    ON generation_task_checkpoint (parent_task_id)
    WHERE parent_task_id IS NOT NULL;
CREATE INDEX idx_generation_task_retry
    ON generation_task_checkpoint (retry_of_task_id)
    WHERE retry_of_task_id IS NOT NULL;

CREATE TABLE generation_start_request (
    requested_by_user_id BIGINT NOT NULL,
    start_request_id UUID NOT NULL,
    task_id TEXT NOT NULL
        REFERENCES task_queue(id) ON DELETE RESTRICT,
    input_fingerprint_sha256 TEXT NOT NULL,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    last_seen_at TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    PRIMARY KEY (requested_by_user_id, start_request_id),
    CONSTRAINT chk_generation_start_request_owner_id CHECK (
        requested_by_user_id BETWEEN 1 AND 9007199254740991
    ),
    CONSTRAINT chk_generation_start_request_uuid_v4 CHECK (
        substring(start_request_id::text, 15, 1) = '4'
        AND substring(start_request_id::text, 20, 1) IN ('8', '9', 'a', 'b')
    ),
    CONSTRAINT chk_generation_start_request_fingerprint CHECK (
        input_fingerprint_sha256 ~ '^[0-9a-f]{64}$'
    ),
    CONSTRAINT chk_generation_start_request_timestamps CHECK (
        last_seen_at >= created_at
    )
);

CREATE INDEX idx_generation_start_request_task
    ON generation_start_request (task_id);
"#;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N generation task persistence migration only supports PostgreSQL".to_string(),
            ));
        }
        manager.get_connection().execute_unprepared(UP_SQL).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "generation_task_persistence 是前向迁移，不能安全回滚".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::UP_SQL;

    #[test]
    fn migration_keeps_generation_persistence_invariants_in_postgres() {
        for required in [
            "generation_task_checkpoint",
            "generation_start_request",
            "uq_generation_task_active_business",
            "reserved_through_sequence BETWEEN covers_through_sequence",
            "chk_generation_task_execution_provider",
            "chk_generation_task_provider_job",
            "REFERENCES task_queue(id) ON DELETE CASCADE",
            "REFERENCES task_queue(id) ON DELETE RESTRICT",
            "REFERENCES generation_task_checkpoint(task_id) ON DELETE SET NULL",
            "ADD COLUMN IF NOT EXISTS lifecycle_gate",
            "conrelid = '\"user\"'::regclass",
            "conrelid = 'novel_project'::regclass",
            "CREATE INDEX IF NOT EXISTS idx_user_lifecycle_gate",
            "lifecycle_gate IN ('active', 'deleting', 'resetting')",
            "lifecycle_gate IN ('active', 'disabling', 'disabled')",
        ] {
            assert!(UP_SQL.contains(required), "migration missing {required}");
        }
        assert_eq!(
            UP_SQL
                .matches("ALTER COLUMN lifecycle_gate SET DEFAULT 'active'")
                .count(),
            2,
            "user and project lifecycle gates must both keep their database default",
        );
    }
}
