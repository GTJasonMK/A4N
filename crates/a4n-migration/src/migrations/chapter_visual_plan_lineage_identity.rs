//! Makes the source snapshot lineage, rather than the derived revision key, the active-plan identity.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N chapter visual plan lineage identity migration only supports PostgreSQL"
                    .into(),
            ));
        }
        let duplicate = manager
            .get_connection()
            .query_one(Statement::from_string(
                DbBackend::Postgres,
                r#"
SELECT root_project_id,
       source_chapter_id,
       source_snapshot_id,
       source_snapshot_chapter_id,
       status,
       COUNT(*) AS duplicate_count
FROM chapter_visual_plan
WHERE status IN ('draft', 'confirmed')
GROUP BY root_project_id,
         source_chapter_id,
         source_snapshot_id,
         source_snapshot_chapter_id,
         status
HAVING COUNT(*) > 1
ORDER BY root_project_id,
         source_chapter_id,
         source_snapshot_id NULLS FIRST,
         source_snapshot_chapter_id NULLS FIRST,
         status
LIMIT 1
"#
                .to_string(),
            ))
            .await?;
        if let Some(row) = duplicate {
            let root_project_id: i64 = row.try_get("", "root_project_id")?;
            let source_chapter_id: i64 = row.try_get("", "source_chapter_id")?;
            let source_snapshot_id: Option<i64> = row.try_get("", "source_snapshot_id")?;
            let source_snapshot_chapter_id: Option<i64> =
                row.try_get("", "source_snapshot_chapter_id")?;
            let status: String = row.try_get("", "status")?;
            let duplicate_count: i64 = row.try_get("", "duplicate_count")?;
            return Err(DbErr::Custom(format!(
                "chapter_visual_plan 存在无法迁移的重复 {status} lineage: root_project_id={root_project_id}, source_chapter_id={source_chapter_id}, source_snapshot_id={source_snapshot_id:?}, source_snapshot_chapter_id={source_snapshot_chapter_id:?}, count={duplicate_count}; 请先人工合并或清理重复记录"
            )));
        }
        manager
            .get_connection()
            .execute_unprepared(
                r#"
DROP INDEX IF EXISTS uidx_chapter_visual_plan_draft;
DROP INDEX IF EXISTS uidx_chapter_visual_plan_confirmed;
DROP INDEX IF EXISTS idx_chapter_visual_plan_source;

ALTER TABLE chapter_visual_plan
    DROP CONSTRAINT IF EXISTS ck_chapter_visual_plan_source_snapshot_identity;
ALTER TABLE chapter_visual_plan
    ADD CONSTRAINT ck_chapter_visual_plan_source_snapshot_identity
    CHECK (
        (source_snapshot_id IS NULL AND source_snapshot_chapter_id IS NULL)
        OR (
            source_snapshot_id IS NOT NULL
            AND source_snapshot_chapter_id IS NOT NULL
            AND source_snapshot_id > 0
            AND source_snapshot_chapter_id > 0
        )
    );

CREATE UNIQUE INDEX uidx_chapter_visual_plan_live_draft
    ON chapter_visual_plan (root_project_id, source_chapter_id)
    WHERE status = 'draft'
      AND source_snapshot_id IS NULL
      AND source_snapshot_chapter_id IS NULL;
CREATE UNIQUE INDEX uidx_chapter_visual_plan_live_confirmed
    ON chapter_visual_plan (root_project_id, source_chapter_id)
    WHERE status = 'confirmed'
      AND source_snapshot_id IS NULL
      AND source_snapshot_chapter_id IS NULL;
CREATE UNIQUE INDEX uidx_chapter_visual_plan_snapshot_draft
    ON chapter_visual_plan (
        root_project_id,
        source_chapter_id,
        source_snapshot_id,
        source_snapshot_chapter_id
    )
    WHERE status = 'draft'
      AND source_snapshot_id IS NOT NULL
      AND source_snapshot_chapter_id IS NOT NULL;
CREATE UNIQUE INDEX uidx_chapter_visual_plan_snapshot_confirmed
    ON chapter_visual_plan (
        root_project_id,
        source_chapter_id,
        source_snapshot_id,
        source_snapshot_chapter_id
    )
    WHERE status = 'confirmed'
      AND source_snapshot_id IS NOT NULL
      AND source_snapshot_chapter_id IS NOT NULL;
CREATE INDEX idx_chapter_visual_plan_lineage
    ON chapter_visual_plan (
        root_project_id,
        source_chapter_id,
        source_snapshot_id,
        source_snapshot_chapter_id,
        status,
        updated_at DESC
    );
"#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "chapter visual plan lineage identity migration is not reversible".into(),
        ))
    }
}
