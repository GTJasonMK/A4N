use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(IDENTITY_SQL)
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "冻结来源的章节身份不能删除或重新变成当前正文".into(),
        ))
    }
}

const IDENTITY_SQL: &str = r#"
DO $$
BEGIN
    IF EXISTS (
        SELECT source_chapter_id FROM source_snapshot_chapter
        GROUP BY source_chapter_id
        HAVING source_chapter_id <= 0 OR MIN(chapter_number) <= 0
            OR COUNT(DISTINCT (root_project_id, chapter_number)) <> 1
    ) OR EXISTS (
        SELECT 1 FROM source_snapshot_chapter s JOIN chapter c ON c.id = s.source_chapter_id
        WHERE (c.project_id, c.chapter_number) IS DISTINCT FROM (s.root_project_id, s.chapter_number)
    ) THEN
        RAISE EXCEPTION '冻结来源的章节身份不一致，无法恢复已删除章节';
    END IF;
END;
$$;

DROP INDEX idx_chapter_project_number_unique;
CREATE UNIQUE INDEX idx_chapter_project_number_unique ON chapter(project_id, chapter_number)
    WHERE status <> 'deleted';

-- Recreate only identities proven by frozen sources; their deleted content stays deleted.
INSERT INTO chapter (id, project_id, chapter_number, content, word_count, status, created_at, updated_at)
SELECT DISTINCT s.source_chapter_id, s.root_project_id, s.chapter_number, '', 0, 'deleted',
       CURRENT_TIMESTAMP AT TIME ZONE 'UTC', CURRENT_TIMESTAMP AT TIME ZONE 'UTC'
FROM source_snapshot_chapter s
WHERE NOT EXISTS (SELECT 1 FROM chapter c WHERE c.id = s.source_chapter_id);

SELECT setval(pg_get_serial_sequence('chapter', 'id'),
    GREATEST((SELECT MAX(id) FROM chapter), nextval(pg_get_serial_sequence('chapter', 'id'))));

CREATE UNIQUE INDEX idx_chapter_source_identity ON chapter(id, project_id, chapter_number);
ALTER TABLE source_snapshot_chapter ADD CONSTRAINT fk_source_snapshot_chapter_identity
    FOREIGN KEY (source_chapter_id, root_project_id, chapter_number)
    REFERENCES chapter(id, project_id, chapter_number) DEFERRABLE INITIALLY DEFERRED;
"#;

#[cfg(test)]
mod tests;
