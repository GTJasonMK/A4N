//! Freeform generation context for video canvas canonical records.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N freeform generation context migration only supports PostgreSQL".to_string(),
            ));
        }

        for sql in FREEFORM_GENERATION_CONTEXT_UP_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N freeform generation context migration only supports PostgreSQL".to_string(),
            ));
        }

        for sql in FREEFORM_GENERATION_CONTEXT_DOWN_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }
}

const FREEFORM_GENERATION_CONTEXT_UP_SQL: &[&str] = &[
    "ALTER TABLE generation_spec ADD COLUMN IF NOT EXISTS source_context_kind TEXT",
    "UPDATE generation_spec SET source_context_kind = 'snapshot_backed' WHERE source_context_kind IS NULL OR source_context_kind = ''",
    "ALTER TABLE generation_spec ADD COLUMN IF NOT EXISTS canvas_text_asset_id BIGINT",
    "ALTER TABLE generation_spec ALTER COLUMN source_snapshot_id DROP NOT NULL",
    "CREATE INDEX IF NOT EXISTS idx_generation_spec_source_context ON generation_spec(source_context_kind)",
    "CREATE INDEX IF NOT EXISTS idx_generation_spec_canvas_text_asset ON generation_spec(canvas_text_asset_id)",
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    context_kind TEXT := COALESCE(NULLIF(NEW.source_context_kind, ''), 'snapshot_backed');
BEGIN
    IF context_kind NOT IN ('snapshot_backed', 'freeform') THEN
        RAISE EXCEPTION '生成意图 source_context_kind 无效';
    END IF;

    IF context_kind = 'snapshot_backed' THEN
        IF NOT EXISTS (
            SELECT 1 FROM novel_project root
            WHERE root.id = NEW.root_project_id
              AND root.project_kind = 'text'
        ) THEN
            RAISE EXCEPTION '生成意图 root_project_id 必须指向文本项目';
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM novel_project project_row
            WHERE project_row.id = NEW.project_id
              AND (
                  (project_row.project_kind = 'text' AND project_row.id = NEW.root_project_id)
                  OR (project_row.project_kind <> 'text' AND project_row.root_project_id = NEW.root_project_id)
              )
        ) THEN
            RAISE EXCEPTION '生成意图 project_id 必须属于同一项目族';
        END IF;
        IF NEW.source_snapshot_id IS NULL THEN
            RAISE EXCEPTION 'snapshot_backed 生成意图必须包含 source_snapshot_id';
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM project_source_snapshot snapshot
            WHERE snapshot.id = NEW.source_snapshot_id
              AND snapshot.root_project_id = NEW.root_project_id
        ) THEN
            RAISE EXCEPTION '生成意图 source_snapshot_id 必须属于根文本项目';
        END IF;
        IF NEW.source_snapshot_chapter_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM source_snapshot_chapter chapter
            WHERE chapter.id = NEW.source_snapshot_chapter_id
              AND chapter.snapshot_id = NEW.source_snapshot_id
              AND chapter.root_project_id = NEW.root_project_id
              AND (NEW.source_chapter_id IS NULL OR chapter.source_chapter_id = NEW.source_chapter_id)
        ) THEN
            RAISE EXCEPTION '生成意图 source_snapshot_chapter_id 必须属于目标快照';
        END IF;
    ELSE
        IF NOT EXISTS (
            SELECT 1 FROM novel_project project_row
            WHERE project_row.id = NEW.project_id
              AND project_row.project_kind = 'video'
              AND project_row.source_mode = 'freeform'
        ) THEN
            RAISE EXCEPTION 'freeform 生成意图必须属于自由视频项目';
        END IF;
        IF NEW.root_project_id <> NEW.project_id THEN
            RAISE EXCEPTION 'freeform 生成意图 root_project_id 必须等于 project_id';
        END IF;
        IF NEW.source_snapshot_id IS NOT NULL
            OR NEW.source_chapter_id IS NOT NULL
            OR NEW.source_snapshot_chapter_id IS NOT NULL
        THEN
            RAISE EXCEPTION 'freeform 生成意图不能绑定源快照或源章节';
        END IF;
        IF NEW.canvas_text_asset_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM canvas_text_asset asset
            WHERE asset.id = NEW.canvas_text_asset_id
              AND asset.project_id = NEW.project_id
        ) THEN
            RAISE EXCEPTION 'freeform 生成意图 canvas_text_asset_id 必须属于当前项目';
        END IF;
    END IF;

    NEW.source_context_kind = context_kind;
    RETURN NEW;
END;
$$
"#,
    "DROP TRIGGER IF EXISTS trg_generation_spec_family_update ON generation_spec",
    r#"
CREATE TRIGGER trg_generation_spec_family_update
BEFORE UPDATE OF project_id, root_project_id, source_context_kind, source_snapshot_id, source_chapter_id, source_snapshot_chapter_id, canvas_text_asset_id ON generation_spec
FOR EACH ROW EXECUTE FUNCTION a4n_validate_generation_spec_family()
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_generation_spec_text_anchor_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM generation_spec spec
        WHERE spec.id = NEW.generation_spec_id
          AND COALESCE(NULLIF(spec.source_context_kind, ''), 'snapshot_backed') = 'freeform'
    ) THEN
        RAISE EXCEPTION 'freeform 生成意图不能绑定 text_anchor';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM generation_spec spec
        JOIN text_anchor anchor ON anchor.id = NEW.text_anchor_id
        WHERE spec.id = NEW.generation_spec_id
          AND anchor.project_id = spec.root_project_id
          AND anchor.source_snapshot_id = spec.source_snapshot_id
    ) THEN
        RAISE EXCEPTION '生成意图文本锚点必须属于同一根项目和源快照';
    END IF;
    RETURN NEW;
END;
$$
"#,
];

const FREEFORM_GENERATION_CONTEXT_DOWN_SQL: &[&str] = &[
    "DELETE FROM generation_spec_text_anchor USING generation_spec WHERE generation_spec_text_anchor.generation_spec_id = generation_spec.id AND COALESCE(NULLIF(generation_spec.source_context_kind, ''), 'snapshot_backed') = 'freeform'",
    "DELETE FROM generation_spec WHERE COALESCE(NULLIF(source_context_kind, ''), 'snapshot_backed') = 'freeform'",
    "UPDATE generation_spec SET source_context_kind = 'snapshot_backed' WHERE source_context_kind IS NULL OR source_context_kind = ''",
    "ALTER TABLE generation_spec ALTER COLUMN source_snapshot_id SET NOT NULL",
    "DROP INDEX IF EXISTS idx_generation_spec_canvas_text_asset",
    "DROP INDEX IF EXISTS idx_generation_spec_source_context",
    "ALTER TABLE generation_spec DROP COLUMN IF EXISTS canvas_text_asset_id",
    "ALTER TABLE generation_spec DROP COLUMN IF EXISTS source_context_kind",
];

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}
