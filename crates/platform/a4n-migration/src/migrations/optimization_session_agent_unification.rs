//! Archive the user-facing optimization sessions retired by the Agent-only UI.

use sea_orm::{ConnectionTrait, DbBackend, DbErr};
use sea_orm_migration::prelude::*;

const UP_SQL: &str = r#"
ALTER TABLE optimization_session
    ADD COLUMN IF NOT EXISTS archived_at TIMESTAMP NULL;
UPDATE optimization_session
SET archived_at = CURRENT_TIMESTAMP
WHERE archived_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_optimization_session_active_lookup
    ON optimization_session (project_id, chapter_id, updated_at DESC, id DESC)
    WHERE archived_at IS NULL;
"#;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N optimization session archival migration only supports PostgreSQL".into(),
            ));
        }
        manager.get_connection().execute_unprepared(UP_SQL).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "optimization_session_agent_unification 不可逆：旧优化会话已归档且必须保留".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::UP_SQL;

    #[test]
    fn archives_existing_sessions_and_indexes_only_active_rows() {
        assert!(UP_SQL.contains("ADD COLUMN IF NOT EXISTS archived_at"));
        assert!(UP_SQL.contains("SET archived_at = CURRENT_TIMESTAMP"));
        assert!(UP_SQL.contains("WHERE archived_at IS NULL"));
    }
}
