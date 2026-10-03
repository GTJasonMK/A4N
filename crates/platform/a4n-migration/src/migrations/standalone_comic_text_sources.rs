//! Persist immutable text-version and title-range identity for standalone comics.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_SQL: &[&str] = &[
    r#"
CREATE TABLE IF NOT EXISTS standalone_comic_text_source (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES novel_project(id) ON DELETE CASCADE,
    chapter_id BIGINT NOT NULL UNIQUE REFERENCES chapter(id) ON DELETE CASCADE,
    source_snapshot_id BIGINT NOT NULL REFERENCES project_source_snapshot(id) ON DELETE CASCADE,
    source_snapshot_chapter_id BIGINT NOT NULL UNIQUE REFERENCES source_snapshot_chapter(id) ON DELETE CASCADE,
    canvas_text_asset_id BIGINT NOT NULL REFERENCES canvas_text_asset(id) ON DELETE RESTRICT,
    text_asset_version INTEGER NOT NULL CHECK (text_asset_version > 0),
    start_section_index INTEGER NOT NULL CHECK (start_section_index >= 0),
    end_section_index INTEGER NOT NULL CHECK (end_section_index >= start_section_index),
    source_title TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    CONSTRAINT standalone_comic_text_source_identity UNIQUE
        (project_id, canvas_text_asset_id, text_asset_version, start_section_index, end_section_index)
)
"#,
    "CREATE INDEX IF NOT EXISTS idx_standalone_comic_text_source_project ON standalone_comic_text_source(project_id, id)",
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_project_source_snapshot_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM novel_project root
        WHERE root.id = NEW.root_project_id
          AND (root.project_kind = 'text' OR (
              root.project_kind = 'comic'
              AND root.root_project_id IS NULL
              AND root.source_snapshot_id IS NULL
          ))
    ) THEN
        RAISE EXCEPTION '来源快照必须属于文本项目或独立漫画项目';
    END IF;
    RETURN NEW;
END $$
"#,
    r#"
CREATE OR REPLACE FUNCTION a4n_validate_source_snapshot_chapter_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM project_source_snapshot snapshot
        WHERE snapshot.id = NEW.snapshot_id
          AND snapshot.root_project_id = NEW.root_project_id
    ) THEN
        RAISE EXCEPTION '来源快照章节必须属于同一项目快照';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM chapter source_chapter
        WHERE source_chapter.id = NEW.source_chapter_id
          AND source_chapter.project_id = NEW.root_project_id
    ) THEN
        RAISE EXCEPTION '来源快照章节必须引用所属项目章节';
    END IF;
    RETURN NEW;
END $$
"#,
];

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "standalone_comic_text_sources only supports PostgreSQL".to_string(),
            ));
        }
        for sql in UP_SQL {
            manager
                .get_connection()
                .execute(Statement::from_string(
                    DbBackend::Postgres,
                    (*sql).to_string(),
                ))
                .await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "standalone_comic_text_sources is irreversible because comic generations reference these frozen source identities".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_identity_is_unique_and_keeps_version_and_range() {
        assert!(UP_SQL[0].contains("canvas_text_asset_id, text_asset_version"));
        assert!(UP_SQL[0].contains("start_section_index, end_section_index"));
        assert!(UP_SQL[0].contains("chapter_id BIGINT NOT NULL UNIQUE"));
    }
}
