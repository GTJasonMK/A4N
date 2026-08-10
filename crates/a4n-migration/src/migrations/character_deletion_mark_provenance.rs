//! Add provenance to character deletion candidates.

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
    "ALTER TABLE protagonist_deletion_mark ADD COLUMN IF NOT EXISTS source_kind TEXT",
    "UPDATE protagonist_deletion_mark SET source_kind = 'legacy_unknown' WHERE source_kind IS NULL OR btrim(source_kind) = ''",
    "ALTER TABLE protagonist_deletion_mark ALTER COLUMN source_kind SET DEFAULT 'legacy_unknown'",
    "ALTER TABLE protagonist_deletion_mark ALTER COLUMN source_kind SET NOT NULL",
    "CREATE INDEX IF NOT EXISTS idx_protagonist_deletion_mark_source_kind ON protagonist_deletion_mark(source_kind)",
];

const DOWN_SQL: &[&str] = &[
    "DROP INDEX IF EXISTS idx_protagonist_deletion_mark_source_kind",
    "ALTER TABLE protagonist_deletion_mark DROP COLUMN IF EXISTS source_kind",
];

fn ensure_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() == DbBackend::Postgres {
        Ok(())
    } else {
        Err(DbErr::Custom(
            "A4N character deletion mark provenance migration only supports PostgreSQL".to_string(),
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

#[cfg(test)]
mod tests {
    use super::UP_SQL;

    #[test]
    fn migration_backfills_unknown_provenance_before_not_null() {
        assert!(UP_SQL[1].contains("legacy_unknown"));
        assert!(UP_SQL[3].contains("SET NOT NULL"));
    }
}
