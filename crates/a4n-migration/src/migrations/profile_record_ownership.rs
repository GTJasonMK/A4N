//! Track current ownership of object and scene profile records.

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
    "ALTER TABLE object_profile ADD COLUMN IF NOT EXISTS ownership_kind TEXT",
    "ALTER TABLE scene_profile ADD COLUMN IF NOT EXISTS ownership_kind TEXT",
    "UPDATE object_profile SET ownership_kind = 'legacy_unknown' WHERE ownership_kind IS NULL OR btrim(ownership_kind) = ''",
    "UPDATE scene_profile SET ownership_kind = 'legacy_unknown' WHERE ownership_kind IS NULL OR btrim(ownership_kind) = ''",
    "ALTER TABLE object_profile ALTER COLUMN ownership_kind SET DEFAULT 'legacy_unknown'",
    "ALTER TABLE scene_profile ALTER COLUMN ownership_kind SET DEFAULT 'legacy_unknown'",
    "ALTER TABLE object_profile ALTER COLUMN ownership_kind SET NOT NULL",
    "ALTER TABLE scene_profile ALTER COLUMN ownership_kind SET NOT NULL",
    "ALTER TABLE object_profile DROP CONSTRAINT IF EXISTS chk_object_profile_ownership_kind",
    "ALTER TABLE object_profile ADD CONSTRAINT chk_object_profile_ownership_kind CHECK (ownership_kind IN ('user', 'ai_sync', 'legacy_unknown'))",
    "ALTER TABLE scene_profile DROP CONSTRAINT IF EXISTS chk_scene_profile_ownership_kind",
    "ALTER TABLE scene_profile ADD CONSTRAINT chk_scene_profile_ownership_kind CHECK (ownership_kind IN ('user', 'ai_sync', 'legacy_unknown'))",
];

const DOWN_SQL: &[&str] = &[
    "ALTER TABLE scene_profile DROP CONSTRAINT IF EXISTS chk_scene_profile_ownership_kind",
    "ALTER TABLE object_profile DROP CONSTRAINT IF EXISTS chk_object_profile_ownership_kind",
    "ALTER TABLE scene_profile DROP COLUMN IF EXISTS ownership_kind",
    "ALTER TABLE object_profile DROP COLUMN IF EXISTS ownership_kind",
];

fn ensure_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() == DbBackend::Postgres {
        Ok(())
    } else {
        Err(DbErr::Custom(
            "A4N profile ownership migration only supports PostgreSQL".to_string(),
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
    fn migration_backfills_unknown_before_enforcing_ownership() {
        assert!(UP_SQL[2].contains("legacy_unknown"));
        assert!(UP_SQL[3].contains("legacy_unknown"));
        assert!(UP_SQL[9].contains("'user', 'ai_sync', 'legacy_unknown'"));
        assert!(UP_SQL[11].contains("'user', 'ai_sync', 'legacy_unknown'"));
    }
}
