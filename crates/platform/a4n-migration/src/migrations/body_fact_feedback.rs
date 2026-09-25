use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Body fact feedback requires PostgreSQL".into(),
            ));
        }
        for sql in [
            "CREATE TABLE IF NOT EXISTS project_fact_feedback (
                project_id BIGINT PRIMARY KEY REFERENCES novel_project(id) ON DELETE CASCADE,
                interval_chapters INTEGER NOT NULL DEFAULT 5 CHECK (interval_chapters > 0),
                consolidated_sources_json JSONB NOT NULL DEFAULT '[]',
                last_consolidated_at TIMESTAMP NULL
            )",
            "CREATE TABLE IF NOT EXISTS material_fact_change (
                id BIGSERIAL PRIMARY KEY,
                project_id BIGINT NOT NULL REFERENCES novel_project(id) ON DELETE CASCADE,
                workflow_run_id BIGINT NULL REFERENCES workflow_run(id) ON DELETE SET NULL,
                chapter_id BIGINT NULL REFERENCES chapter(id) ON DELETE SET NULL,
                change_key TEXT NOT NULL,
                material_kind TEXT NOT NULL,
                target_id BIGINT NULL,
                field_path TEXT NOT NULL,
                operation TEXT NOT NULL CHECK (operation IN ('add', 'replace', 'remove')),
                before_json JSONB NOT NULL,
                after_json JSONB NOT NULL,
                source_refs_json JSONB NOT NULL,
                evidence_json JSONB NOT NULL,
                reason TEXT NOT NULL,
                status TEXT NOT NULL CHECK (status IN ('applied', 'proposal', 'retracted')),
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                invalidated_at TIMESTAMP NULL,
                UNIQUE(project_id, change_key)
            )",
            "CREATE INDEX IF NOT EXISTS idx_material_fact_change_target ON material_fact_change(project_id, material_kind, target_id, id)",
            "CREATE INDEX IF NOT EXISTS idx_material_fact_change_sources ON material_fact_change USING GIN(source_refs_json)",
        ] {
            manager
                .get_connection()
                .execute(Statement::from_string(DbBackend::Postgres, sql))
                .await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "正文事实日志包含资料来源，不能通过降级迁移删除；请恢复升级前备份".into(),
        ))
    }
}
