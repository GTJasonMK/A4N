//! Persist provenance for automatically derived chapter outlines.

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
    r#"
CREATE TABLE IF NOT EXISTS chapter_outline_provenance (
    outline_id BIGINT PRIMARY KEY,
    source_kind TEXT NOT NULL,
    source_fingerprint_json JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
)
"#,
    // Repair legacy soft links before the provenance contract is enabled.  A
    // matching project/chapter outline is the only safe replacement; all
    // remaining deleted/cross-project links become NULL.
    r#"
UPDATE chapter AS c
SET outline_id = o.id
FROM chapter_outline AS o
WHERE c.outline_id IS NOT NULL
  AND o.project_id = c.project_id
  AND o.chapter_number = c.chapter_number
  AND c.outline_id <> o.id
"#,
    r#"
UPDATE chapter AS c
SET outline_id = NULL
WHERE c.outline_id IS NOT NULL
  AND NOT EXISTS (
      SELECT 1
      FROM chapter_outline AS o
      WHERE o.id = c.outline_id
        AND o.project_id = c.project_id
        AND o.chapter_number = c.chapter_number
  )
"#,
    "ALTER TABLE chapter_outline_provenance DROP CONSTRAINT IF EXISTS \"fk-chapter_outline_provenance-outline_id\"",
    "ALTER TABLE chapter_outline_provenance DROP CONSTRAINT IF EXISTS fk_chapter_outline_provenance_outline",
    r#"
ALTER TABLE chapter_outline_provenance
    ADD CONSTRAINT "fk-chapter_outline_provenance-outline_id"
    FOREIGN KEY (outline_id) REFERENCES chapter_outline(id) ON DELETE CASCADE
"#,
    "CREATE INDEX IF NOT EXISTS idx_chapter_outline_provenance_source_kind ON chapter_outline_provenance(source_kind)",
];

const DOWN_SQL: &[&str] = &[
    "DROP INDEX IF EXISTS idx_chapter_outline_provenance_source_kind",
    "DROP TABLE IF EXISTS chapter_outline_provenance",
];

fn ensure_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() != DbBackend::Postgres {
        return Err(DbErr::Custom(
            "A4N chapter outline provenance migration only supports PostgreSQL".to_string(),
        ));
    }
    Ok(())
}

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}
