//! Retires the old automatic visual-plan authoring mode.

use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N chapter visual plan authoring mode cleanup migration only supports PostgreSQL"
                    .into(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(
                "UPDATE chapter_visual_plan
                    SET authoring_mode = 'collaborative'
                  WHERE authoring_mode = 'automatic';
                 ALTER TABLE chapter_visual_plan
                    DROP CONSTRAINT IF EXISTS ck_chapter_visual_plan_authoring_mode;
                 ALTER TABLE chapter_visual_plan
                    ADD CONSTRAINT ck_chapter_visual_plan_authoring_mode
                    CHECK (authoring_mode IN ('collaborative', 'manual'));",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "chapter visual plan authoring mode cleanup is not reversible".into(),
        ))
    }
}
