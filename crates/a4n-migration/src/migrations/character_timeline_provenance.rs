//! Add provenance to character timeline records.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_postgres(manager)?;
        for sql in UP_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_postgres(manager)?;
        for sql in DOWN_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }
}

const UP_SQL: &[&str] = &[
    "ALTER TABLE attribute_event ADD COLUMN IF NOT EXISTS source_kind TEXT",
    "ALTER TABLE protagonist_behavior_record ADD COLUMN IF NOT EXISTS source_kind TEXT",
    "UPDATE attribute_event SET source_kind = 'legacy_unknown' WHERE source_kind IS NULL OR btrim(source_kind) = ''",
    "UPDATE protagonist_behavior_record SET source_kind = 'legacy_unknown' WHERE source_kind IS NULL OR btrim(source_kind) = ''",
    "ALTER TABLE attribute_event ALTER COLUMN source_kind SET DEFAULT 'legacy_unknown'",
    "ALTER TABLE protagonist_behavior_record ALTER COLUMN source_kind SET DEFAULT 'legacy_unknown'",
    "ALTER TABLE attribute_event ALTER COLUMN source_kind SET NOT NULL",
    "ALTER TABLE protagonist_behavior_record ALTER COLUMN source_kind SET NOT NULL",
    "CREATE INDEX IF NOT EXISTS idx_attribute_event_source_kind ON attribute_event(source_kind)",
    "CREATE INDEX IF NOT EXISTS idx_protagonist_behavior_record_source_kind ON protagonist_behavior_record(source_kind)",
];

const DOWN_SQL: &[&str] = &[
    "DROP INDEX IF EXISTS idx_protagonist_behavior_record_source_kind",
    "DROP INDEX IF EXISTS idx_attribute_event_source_kind",
    "ALTER TABLE protagonist_behavior_record DROP COLUMN IF EXISTS source_kind",
    "ALTER TABLE attribute_event DROP COLUMN IF EXISTS source_kind",
];

fn ensure_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() == DbBackend::Postgres {
        Ok(())
    } else {
        Err(DbErr::Custom(
            "A4N character timeline provenance migration only supports PostgreSQL".to_string(),
        ))
    }
}

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}
