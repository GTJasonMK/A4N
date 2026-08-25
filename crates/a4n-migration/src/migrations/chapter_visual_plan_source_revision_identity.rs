//! Separates live and snapshot visual plans even when their content hashes match.

use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N chapter visual plan source identity migration only supports PostgreSQL".into(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(
                r#"
DROP INDEX IF EXISTS uidx_chapter_visual_plan_draft;
DROP INDEX IF EXISTS uidx_chapter_visual_plan_confirmed;
DROP INDEX IF EXISTS idx_chapter_visual_plan_source;

CREATE UNIQUE INDEX uidx_chapter_visual_plan_draft
    ON chapter_visual_plan (root_project_id, source_chapter_id, source_revision_key)
    WHERE status = 'draft';
CREATE UNIQUE INDEX uidx_chapter_visual_plan_confirmed
    ON chapter_visual_plan (root_project_id, source_chapter_id, source_revision_key)
    WHERE status = 'confirmed';
CREATE INDEX idx_chapter_visual_plan_source
    ON chapter_visual_plan (root_project_id, source_chapter_id, source_revision_key, status);
"#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "chapter visual plan source identity migration is not reversible".into(),
        ))
    }
}
