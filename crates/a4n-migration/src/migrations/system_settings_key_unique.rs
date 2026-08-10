//! Enforce the global identity of `system_settings.key`.

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const DEDUPLICATE_SQL: &str = r#"
WITH ranked AS (
    SELECT
        id,
        ROW_NUMBER() OVER (
            PARTITION BY key
            ORDER BY updated_at DESC NULLS LAST, id DESC
        ) AS row_number
    FROM system_settings
)
DELETE FROM system_settings AS setting
USING ranked
WHERE setting.id = ranked.id
  AND ranked.row_number > 1;
"#;

const CREATE_INDEX_SQL: &str =
    "CREATE UNIQUE INDEX IF NOT EXISTS uidx_system_settings_key ON system_settings (key);";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_postgres(manager)?;
        manager
            .get_connection()
            .execute_unprepared(DEDUPLICATE_SQL)
            .await?;
        manager
            .get_connection()
            .execute_unprepared(CREATE_INDEX_SQL)
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "system_settings_key_unique 不可逆：已删除的重复设置记录无法安全恢复".to_string(),
        ))
    }
}

fn ensure_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() == DbBackend::Postgres {
        Ok(())
    } else {
        Err(DbErr::Custom(
            "A4N system_settings key migration only supports PostgreSQL".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{CREATE_INDEX_SQL, DEDUPLICATE_SQL};

    #[test]
    fn deduplication_keeps_latest_updated_at_then_id() {
        assert!(DEDUPLICATE_SQL.contains("PARTITION BY key"));
        assert!(DEDUPLICATE_SQL.contains("ORDER BY updated_at DESC NULLS LAST, id DESC"));
        assert!(DEDUPLICATE_SQL.contains("row_number > 1"));
    }

    #[test]
    fn index_creation_is_idempotent() {
        assert!(CREATE_INDEX_SQL.contains("CREATE UNIQUE INDEX IF NOT EXISTS"));
        assert!(CREATE_INDEX_SQL.contains("uidx_system_settings_key"));
    }
}
