//! Adds the confirmed director-authored visual plan between source text and shooting script.

use a4n_core::models::chapter::chapter_visual_plan;
use sea_orm::{ConnectionTrait, DbBackend, Schema, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N chapter visual plan migration only supports PostgreSQL".into(),
            ));
        }
        let schema = Schema::new(DbBackend::Postgres);
        let mut table = schema.create_table_from_entity(chapter_visual_plan::Entity);
        table.if_not_exists();
        manager.create_table(table).await?;

        for sql in [
            "ALTER TABLE chapter_shooting_script ADD COLUMN IF NOT EXISTS visual_plan_id BIGINT NULL",
            "ALTER TABLE chapter_shooting_script ADD COLUMN IF NOT EXISTS visual_plan_version INTEGER NULL",
            "ALTER TABLE chapter_shooting_script ADD COLUMN IF NOT EXISTS visual_plan_sha256 VARCHAR NULL",
            "ALTER TABLE source_snapshot_chapter ADD COLUMN IF NOT EXISTS visual_plan_id BIGINT NULL",
            "ALTER TABLE source_snapshot_chapter ADD COLUMN IF NOT EXISTS visual_plan_version INTEGER NULL",
            "ALTER TABLE source_snapshot_chapter ADD COLUMN IF NOT EXISTS visual_plan_sha256 VARCHAR NULL",
            "CREATE UNIQUE INDEX IF NOT EXISTS uidx_chapter_visual_plan_draft ON chapter_visual_plan (root_project_id, source_chapter_id, source_revision_key) WHERE status = 'draft'",
            "CREATE UNIQUE INDEX IF NOT EXISTS uidx_chapter_visual_plan_confirmed ON chapter_visual_plan (root_project_id, source_chapter_id, source_revision_key) WHERE status = 'confirmed'",
            "CREATE UNIQUE INDEX IF NOT EXISTS uidx_chapter_visual_plan_version ON chapter_visual_plan (root_project_id, source_chapter_id, plan_version)",
            "CREATE INDEX IF NOT EXISTS idx_chapter_visual_plan_source ON chapter_visual_plan (root_project_id, source_chapter_id, source_revision_key, status)",
            "CREATE INDEX IF NOT EXISTS idx_chapter_visual_plan_snapshot_chapter ON chapter_visual_plan (source_snapshot_chapter_id)",
            "ALTER TABLE chapter_visual_plan DROP CONSTRAINT IF EXISTS ck_chapter_visual_plan_status",
            "ALTER TABLE chapter_visual_plan ADD CONSTRAINT ck_chapter_visual_plan_status CHECK (status IN ('draft', 'confirmed', 'superseded', 'stale'))",
            "ALTER TABLE chapter_visual_plan DROP CONSTRAINT IF EXISTS ck_chapter_visual_plan_authoring_mode",
            "ALTER TABLE chapter_visual_plan ADD CONSTRAINT ck_chapter_visual_plan_authoring_mode CHECK (authoring_mode IN ('automatic', 'collaborative', 'manual'))",
            "ALTER TABLE chapter_visual_plan DROP CONSTRAINT IF EXISTS ck_chapter_visual_plan_positive_version",
            "ALTER TABLE chapter_visual_plan ADD CONSTRAINT ck_chapter_visual_plan_positive_version CHECK (plan_version > 0 AND revision > 0)",
            "ALTER TABLE chapter_visual_plan DROP CONSTRAINT IF EXISTS ck_chapter_visual_plan_confirmed_at",
            "ALTER TABLE chapter_visual_plan ADD CONSTRAINT ck_chapter_visual_plan_confirmed_at CHECK ((status = 'confirmed' AND confirmed_at IS NOT NULL) OR status <> 'confirmed')",
            "ALTER TABLE chapter_shooting_script DROP CONSTRAINT IF EXISTS ck_chapter_shooting_script_visual_plan_identity",
            "ALTER TABLE chapter_shooting_script ADD CONSTRAINT ck_chapter_shooting_script_visual_plan_identity CHECK ((visual_plan_id IS NULL AND visual_plan_version IS NULL AND visual_plan_sha256 IS NULL) OR (visual_plan_id IS NOT NULL AND visual_plan_version > 0 AND visual_plan_sha256 IS NOT NULL))",
            "ALTER TABLE source_snapshot_chapter DROP CONSTRAINT IF EXISTS ck_source_snapshot_chapter_visual_plan_identity",
            "ALTER TABLE source_snapshot_chapter ADD CONSTRAINT ck_source_snapshot_chapter_visual_plan_identity CHECK ((visual_plan_id IS NULL AND visual_plan_version IS NULL AND visual_plan_sha256 IS NULL) OR (visual_plan_id IS NOT NULL AND visual_plan_version > 0 AND visual_plan_sha256 IS NOT NULL))",
        ] {
            manager
                .get_connection()
                .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N chapter visual plan migration only supports PostgreSQL".into(),
            ));
        }
        for sql in [
            "ALTER TABLE chapter_shooting_script DROP CONSTRAINT IF EXISTS ck_chapter_shooting_script_visual_plan_identity",
            "ALTER TABLE chapter_shooting_script DROP COLUMN IF EXISTS visual_plan_sha256",
            "ALTER TABLE chapter_shooting_script DROP COLUMN IF EXISTS visual_plan_version",
            "ALTER TABLE chapter_shooting_script DROP COLUMN IF EXISTS visual_plan_id",
            "ALTER TABLE source_snapshot_chapter DROP CONSTRAINT IF EXISTS ck_source_snapshot_chapter_visual_plan_identity",
            "ALTER TABLE source_snapshot_chapter DROP COLUMN IF EXISTS visual_plan_sha256",
            "ALTER TABLE source_snapshot_chapter DROP COLUMN IF EXISTS visual_plan_version",
            "ALTER TABLE source_snapshot_chapter DROP COLUMN IF EXISTS visual_plan_id",
            "DROP TABLE IF EXISTS chapter_visual_plan",
        ] {
            manager
                .get_connection()
                .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
                .await?;
        }
        Ok(())
    }
}
