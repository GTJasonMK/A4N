//! Add snapshot-level context to canonical media assets.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N media asset snapshot context migration only supports PostgreSQL".to_string(),
            ));
        }

        for sql in MEDIA_ASSET_SNAPSHOT_CONTEXT_UP_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N media asset snapshot context migration only supports PostgreSQL".to_string(),
            ));
        }

        for sql in MEDIA_ASSET_SNAPSHOT_CONTEXT_DOWN_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }
}

const MEDIA_ASSET_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_media_asset_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    project_kind TEXT;
    project_source_mode TEXT;
    project_source_snapshot_id BIGINT;
BEGIN
    SELECT project_row.project_kind,
           project_row.source_mode,
           project_row.source_snapshot_id
      INTO project_kind,
           project_source_mode,
           project_source_snapshot_id
    FROM novel_project project_row
    WHERE project_row.id = NEW.project_id;

    IF project_kind IS NULL THEN
        RAISE EXCEPTION '媒体资产 project_id 不存在';
    END IF;

    IF project_source_mode = 'freeform' THEN
        IF NEW.root_project_id <> NEW.project_id THEN
            RAISE EXCEPTION 'freeform 媒体资产 root_project_id 必须等于 project_id';
        END IF;
        IF NEW.source_snapshot_id IS NOT NULL
           OR NEW.source_chapter_id IS NOT NULL
           OR NEW.source_snapshot_chapter_id IS NOT NULL THEN
            RAISE EXCEPTION 'freeform 媒体资产不能绑定源快照或源章节';
        END IF;
    ELSE
        IF NOT EXISTS (
            SELECT 1 FROM novel_project root
            WHERE root.id = NEW.root_project_id
              AND root.project_kind = 'text'
              AND root.source_mode = 'self'
        ) THEN
            RAISE EXCEPTION '媒体资产 root_project_id 必须指向文本项目';
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM novel_project project_row
            WHERE project_row.id = NEW.project_id
              AND (
                  (project_row.project_kind = 'text' AND project_row.id = NEW.root_project_id)
                  OR (
                      project_row.project_kind <> 'text'
                      AND project_row.source_mode = 'derived'
                      AND project_row.root_project_id = NEW.root_project_id
                  )
              )
        ) THEN
            RAISE EXCEPTION '媒体资产 project_id 必须属于同一项目族';
        END IF;
        IF NEW.source_snapshot_id IS NULL THEN
            RAISE EXCEPTION 'snapshot-backed 媒体资产必须包含 source_snapshot_id';
        END IF;
        IF project_kind <> 'text'
           AND project_source_snapshot_id IS DISTINCT FROM NEW.source_snapshot_id THEN
            RAISE EXCEPTION '媒体资产 source_snapshot_id 必须等于派生项目源快照';
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM project_source_snapshot snapshot
            WHERE snapshot.id = NEW.source_snapshot_id
              AND snapshot.root_project_id = NEW.root_project_id
        ) THEN
            RAISE EXCEPTION '媒体资产 source_snapshot_id 必须属于根文本项目';
        END IF;
        IF NEW.source_chapter_id IS NOT NULL AND NEW.source_snapshot_chapter_id IS NULL THEN
            RAISE EXCEPTION '设置 source_chapter_id 时必须同时设置 source_snapshot_chapter_id';
        END IF;
        IF NEW.source_snapshot_chapter_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM source_snapshot_chapter chapter
            WHERE chapter.id = NEW.source_snapshot_chapter_id
              AND chapter.root_project_id = NEW.root_project_id
              AND chapter.snapshot_id = NEW.source_snapshot_id
              AND (NEW.source_chapter_id IS NULL OR chapter.source_chapter_id = NEW.source_chapter_id)
        ) THEN
            RAISE EXCEPTION '媒体资产 source_snapshot_chapter_id 必须属于同一源快照章节';
        END IF;
    END IF;

    IF NEW.status = 'completed' AND NEW.file_path IS NULL THEN
        RAISE EXCEPTION 'completed 媒体资产必须有 file_path';
    END IF;
    RETURN NEW;
END;
$$
"#;

const MEDIA_ASSET_FAMILY_LEGACY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_media_asset_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    project_source_mode TEXT;
BEGIN
    SELECT project_row.source_mode INTO project_source_mode
    FROM novel_project project_row
    WHERE project_row.id = NEW.project_id
      AND project_row.project_kind = 'video';

    IF project_source_mode = 'freeform' THEN
        IF NEW.root_project_id <> NEW.project_id THEN
            RAISE EXCEPTION 'freeform 媒体资产 root_project_id 必须等于 project_id';
        END IF;
        IF NEW.source_chapter_id IS NOT NULL OR NEW.source_snapshot_chapter_id IS NOT NULL THEN
            RAISE EXCEPTION 'freeform 媒体资产不能绑定源章节或源快照章节';
        END IF;
    ELSE
        IF NOT EXISTS (
            SELECT 1 FROM novel_project root
            WHERE root.id = NEW.root_project_id
              AND root.project_kind = 'text'
              AND root.source_mode = 'self'
        ) THEN
            RAISE EXCEPTION '媒体资产 root_project_id 必须指向文本项目';
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM novel_project project_row
            WHERE project_row.id = NEW.project_id
              AND (
                  (project_row.project_kind = 'text' AND project_row.id = NEW.root_project_id)
                  OR (
                      project_row.project_kind <> 'text'
                      AND project_row.source_mode = 'derived'
                      AND project_row.root_project_id = NEW.root_project_id
                  )
              )
        ) THEN
            RAISE EXCEPTION '媒体资产 project_id 必须属于同一项目族';
        END IF;
        IF NEW.source_snapshot_chapter_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM source_snapshot_chapter chapter
            WHERE chapter.id = NEW.source_snapshot_chapter_id
              AND chapter.root_project_id = NEW.root_project_id
              AND (NEW.source_chapter_id IS NULL OR chapter.source_chapter_id = NEW.source_chapter_id)
        ) THEN
            RAISE EXCEPTION '媒体资产 source_snapshot_chapter_id 必须属于根文本快照章节';
        END IF;
    END IF;
    IF NEW.status = 'completed' AND NEW.file_path IS NULL THEN
        RAISE EXCEPTION 'completed 媒体资产必须有 file_path';
    END IF;
    RETURN NEW;
END;
$$
"#;

const MEDIA_ASSET_SNAPSHOT_CONTEXT_UP_SQL: &[&str] = &[
    "ALTER TABLE media_asset ADD COLUMN IF NOT EXISTS source_snapshot_id BIGINT",
    r#"
UPDATE media_asset asset
SET source_snapshot_id = COALESCE(
    (
        SELECT chapter.snapshot_id
        FROM source_snapshot_chapter chapter
        WHERE chapter.id = asset.source_snapshot_chapter_id
    ),
    (
        SELECT project_row.source_snapshot_id
        FROM novel_project project_row
        WHERE project_row.id = asset.project_id
    )
)
WHERE asset.source_snapshot_id IS NULL
  AND (
      EXISTS (
          SELECT 1
          FROM source_snapshot_chapter chapter
          WHERE chapter.id = asset.source_snapshot_chapter_id
            AND chapter.snapshot_id IS NOT NULL
      )
      OR EXISTS (
          SELECT 1
          FROM novel_project project_row
          WHERE project_row.id = asset.project_id
            AND project_row.source_snapshot_id IS NOT NULL
      )
  )
"#,
    "CREATE INDEX IF NOT EXISTS idx_media_asset_source_snapshot ON media_asset (source_snapshot_id)",
    MEDIA_ASSET_FAMILY_FUNCTION_SQL,
    "DROP TRIGGER IF EXISTS trg_media_asset_family_update ON media_asset",
    r#"
CREATE TRIGGER trg_media_asset_family_update
BEFORE UPDATE OF project_id, root_project_id, source_snapshot_id, source_chapter_id, source_snapshot_chapter_id, status, file_path ON media_asset
FOR EACH ROW EXECUTE FUNCTION a4n_validate_media_asset_family()
"#,
];

const MEDIA_ASSET_SNAPSHOT_CONTEXT_DOWN_SQL: &[&str] = &[
    "DROP INDEX IF EXISTS idx_media_asset_source_snapshot",
    "DROP TRIGGER IF EXISTS trg_media_asset_family_update ON media_asset",
    MEDIA_ASSET_FAMILY_LEGACY_FUNCTION_SQL,
    r#"
CREATE TRIGGER trg_media_asset_family_update
BEFORE UPDATE OF project_id, root_project_id, source_chapter_id, source_snapshot_chapter_id, status, file_path ON media_asset
FOR EACH ROW EXECUTE FUNCTION a4n_validate_media_asset_family()
"#,
    "ALTER TABLE media_asset DROP COLUMN IF EXISTS source_snapshot_id",
];

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}
