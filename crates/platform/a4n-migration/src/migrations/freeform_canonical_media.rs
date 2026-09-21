//! Freeform canonical media and assembly contracts for video canvas.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N freeform canonical media migration only supports PostgreSQL".to_string(),
            ));
        }

        for sql in FREEFORM_CANONICAL_MEDIA_UP_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N freeform canonical media migration only supports PostgreSQL".to_string(),
            ));
        }

        for sql in FREEFORM_CANONICAL_MEDIA_DOWN_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }
}

pub(crate) const VIDEO_SEGMENT_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_video_segment_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    project_kind TEXT;
    project_root_project_id BIGINT;
    project_source_snapshot_id BIGINT;
BEGIN
    SELECT project_row.project_kind,
           project_row.root_project_id,
           project_row.source_snapshot_id
      INTO project_kind,
           project_root_project_id,
           project_source_snapshot_id
    FROM novel_project project_row
    WHERE project_row.id = NEW.project_id;

    IF project_kind = 'video'
       AND project_root_project_id IS NULL
       AND project_source_snapshot_id IS NULL THEN
        IF NEW.root_project_id <> NEW.project_id THEN
            RAISE EXCEPTION 'freeform 视频片段 root_project_id 必须等于 project_id';
        END IF;
        IF NEW.source_chapter_id IS NOT NULL OR NEW.source_snapshot_chapter_id IS NOT NULL THEN
            RAISE EXCEPTION 'freeform 视频片段不能绑定源章节或源快照章节';
        END IF;
    ELSIF project_kind = 'video'
          AND project_root_project_id IS NOT NULL
          AND project_source_snapshot_id IS NOT NULL THEN
        IF project_root_project_id IS DISTINCT FROM NEW.root_project_id THEN
            RAISE EXCEPTION '视频片段 project_id 必须指向派生视频项目';
        END IF;
        IF NEW.source_snapshot_chapter_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM source_snapshot_chapter chapter
            WHERE chapter.id = NEW.source_snapshot_chapter_id
              AND chapter.root_project_id = NEW.root_project_id
              AND (NEW.source_chapter_id IS NULL OR chapter.source_chapter_id = NEW.source_chapter_id)
        ) THEN
            RAISE EXCEPTION '视频片段 source_snapshot_chapter_id 必须属于根文本快照章节';
        END IF;
    ELSE
        RAISE EXCEPTION '视频片段 project_id 必须指向视频项目';
    END IF;

    IF NEW.source_generation_spec_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM generation_spec spec
        WHERE spec.id = NEW.source_generation_spec_id
          AND spec.project_id = NEW.project_id
          AND spec.root_project_id = NEW.root_project_id
    ) THEN
        RAISE EXCEPTION '视频片段 source_generation_spec_id 必须属于同一视频项目';
    END IF;
    IF NEW.result_media_asset_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM media_asset asset
        WHERE asset.id = NEW.result_media_asset_id
          AND asset.project_id = NEW.project_id
          AND asset.root_project_id = NEW.root_project_id
          AND asset.media_kind = 'video'
    ) THEN
        RAISE EXCEPTION '视频片段 result_media_asset_id 必须是同项目视频资产';
    END IF;
    RETURN NEW;
END;
$$
"#;

pub(crate) const ASSEMBLY_TIMELINE_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_assembly_timeline_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    project_kind TEXT;
    project_root_project_id BIGINT;
    project_source_snapshot_id BIGINT;
BEGIN
    SELECT project_row.project_kind,
           project_row.root_project_id,
           project_row.source_snapshot_id
      INTO project_kind,
           project_root_project_id,
           project_source_snapshot_id
    FROM novel_project project_row
    WHERE project_row.id = NEW.project_id;

    IF project_kind = 'video'
       AND project_root_project_id IS NULL
       AND project_source_snapshot_id IS NULL THEN
        IF NEW.source_chapter_id IS NOT NULL OR NEW.source_snapshot_chapter_id IS NOT NULL THEN
            RAISE EXCEPTION 'freeform 成片时间线不能绑定源章节或源快照章节';
        END IF;
    ELSIF project_kind = 'video'
          AND project_root_project_id IS NOT NULL
          AND project_source_snapshot_id IS NOT NULL THEN
        IF NEW.source_snapshot_chapter_id IS NOT NULL AND NOT EXISTS (
            SELECT 1
            FROM novel_project project_row
            JOIN source_snapshot_chapter chapter ON chapter.id = NEW.source_snapshot_chapter_id
            WHERE project_row.id = NEW.project_id
              AND chapter.root_project_id = project_row.root_project_id
              AND (NEW.source_chapter_id IS NULL OR chapter.source_chapter_id = NEW.source_chapter_id)
        ) THEN
            RAISE EXCEPTION '成片时间线 source_snapshot_chapter_id 必须属于视频项目根文本';
        END IF;
    ELSE
        RAISE EXCEPTION '成片时间线 project_id 必须指向视频项目';
    END IF;

    IF NEW.preview_media_asset_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM media_asset asset
        WHERE asset.id = NEW.preview_media_asset_id
          AND asset.project_id = NEW.project_id
          AND asset.media_kind = 'video'
          AND asset.origin = 'assembled_preview'
          AND asset.status = 'completed'
    ) THEN
        RAISE EXCEPTION '成片时间线 preview_media_asset_id 必须是同项目成片预览资产';
    END IF;
    RETURN NEW;
END;
$$
"#;

const FREEFORM_CANONICAL_MEDIA_UP_SQL: &[&str] = &[
    r#"
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
"#,
    VIDEO_SEGMENT_FAMILY_FUNCTION_SQL,
    ASSEMBLY_TIMELINE_FAMILY_FUNCTION_SQL,
];

const FREEFORM_CANONICAL_MEDIA_DOWN_SQL: &[&str] = &[];

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}
