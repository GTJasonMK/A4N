//! 允许 terminal generation snapshot 到期后清除正文但保留 hash/结果引用。

use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

const UP_SQL: &str = r#"
ALTER TABLE generation_task_checkpoint
    DROP CONSTRAINT chk_generation_task_public_snapshot,
    ADD CONSTRAINT chk_generation_task_public_snapshot CHECK (
        public_snapshot_sha256 IS NOT NULL
        AND
        public_snapshot_sha256 ~ '^[0-9a-f]{64}$'
        AND (
            public_snapshot_json IS NOT NULL
            OR (
                status IN ('completed', 'failed', 'cancelled', 'interrupted')
                AND expires_at IS NOT NULL
            )
        )
    );
"#;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "generation snapshot retention migration 仅支持 PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(UP_SQL)
            .await
            .map(|_| ())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "generation_snapshot_retention 是前向迁移，不能安全回滚".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::UP_SQL;

    #[test]
    fn migration_keeps_terminal_hash_after_body_expiry() {
        assert!(UP_SQL.contains("public_snapshot_json IS NOT NULL"));
        assert!(UP_SQL.contains("public_snapshot_sha256 ~ '^[0-9a-f]{64}$'"));
        assert!(UP_SQL.contains("expires_at IS NOT NULL"));
    }
}
