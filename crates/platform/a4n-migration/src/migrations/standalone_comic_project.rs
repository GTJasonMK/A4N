//! Allow comic projects without a novel source snapshot.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const VALIDATE_FAMILY_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_novel_project_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.project_kind NOT IN ('text','comic','video','galgame') THEN
        RAISE EXCEPTION 'novel_project.project_kind 无效';
    END IF;
    IF NOT (
        (NEW.project_kind = 'text'
            AND NEW.root_project_id IS NULL
            AND NEW.source_snapshot_id IS NULL)
        OR (NEW.project_kind = 'video'
            AND NEW.root_project_id IS NULL
            AND NEW.source_snapshot_id IS NULL)
        OR (NEW.project_kind = 'comic'
            AND NEW.root_project_id IS NULL
            AND NEW.source_snapshot_id IS NULL)
        OR (NEW.project_kind IN ('comic','video','galgame')
            AND NEW.root_project_id IS NOT NULL
            AND NEW.source_snapshot_id IS NOT NULL)
    ) THEN
        RAISE EXCEPTION 'novel_project 项目来源身份无效';
    END IF;
    IF NEW.root_project_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM novel_project root
        WHERE root.id = NEW.root_project_id
          AND root.project_kind = 'text'
          AND root.root_project_id IS NULL
          AND root.source_snapshot_id IS NULL
    ) THEN
        RAISE EXCEPTION '派生项目 root_project_id 必须指向文本项目';
    END IF;
    IF NEW.source_snapshot_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM project_source_snapshot snapshot
        WHERE snapshot.id = NEW.source_snapshot_id
          AND snapshot.root_project_id = NEW.root_project_id
    ) THEN
        RAISE EXCEPTION '派生项目 source_snapshot_id 必须属于 root_project_id';
    END IF;
    RETURN NEW;
END;
$$
"#;

const RESTORE_FAMILY_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_novel_project_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.project_kind NOT IN ('text','comic','video','galgame') THEN
        RAISE EXCEPTION 'novel_project.project_kind 无效';
    END IF;
    IF NOT (
        (NEW.project_kind = 'text'
            AND NEW.root_project_id IS NULL
            AND NEW.source_snapshot_id IS NULL)
        OR (NEW.project_kind = 'video'
            AND NEW.root_project_id IS NULL
            AND NEW.source_snapshot_id IS NULL)
        OR (NEW.project_kind IN ('comic','video','galgame')
            AND NEW.root_project_id IS NOT NULL
            AND NEW.source_snapshot_id IS NOT NULL)
    ) THEN
        RAISE EXCEPTION 'novel_project 项目来源身份无效';
    END IF;
    IF NEW.root_project_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM novel_project root
        WHERE root.id = NEW.root_project_id
          AND root.project_kind = 'text'
          AND root.root_project_id IS NULL
          AND root.source_snapshot_id IS NULL
    ) THEN
        RAISE EXCEPTION '派生项目 root_project_id 必须指向文本项目';
    END IF;
    IF NEW.source_snapshot_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM project_source_snapshot snapshot
        WHERE snapshot.id = NEW.source_snapshot_id
          AND snapshot.root_project_id = NEW.root_project_id
    ) THEN
        RAISE EXCEPTION '派生项目 source_snapshot_id 必须属于 root_project_id';
    END IF;
    RETURN NEW;
END;
$$
"#;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute_pg_sql(manager, VALIDATE_FAMILY_SQL).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "standalone_comic_project only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(
                "DO $$ BEGIN IF EXISTS (SELECT 1 FROM novel_project WHERE project_kind='comic' AND root_project_id IS NULL AND source_snapshot_id IS NULL) THEN RAISE EXCEPTION 'standalone comic projects exist'; END IF; END $$",
            )
            .await?;
        execute_pg_sql(manager, RESTORE_FAMILY_SQL).await
    }
}

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    if manager.get_database_backend() != DbBackend::Postgres {
        return Err(DbErr::Custom(
            "standalone_comic_project only supports PostgreSQL".to_string(),
        ));
    }
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_contract_accepts_standalone_and_derived_comic_projects() {
        assert!(VALIDATE_FAMILY_SQL.contains("NEW.project_kind = 'comic'"));
        assert!(VALIDATE_FAMILY_SQL.contains("AND NEW.source_snapshot_id IS NULL"));
        assert!(VALIDATE_FAMILY_SQL.contains("root_project_id IS NOT NULL"));
        assert!(RESTORE_FAMILY_SQL.contains("project_kind IN ('comic','video','galgame')"));
        assert!(!RESTORE_FAMILY_SQL.contains("NEW.project_kind = 'comic'\n            AND"));
    }
}
