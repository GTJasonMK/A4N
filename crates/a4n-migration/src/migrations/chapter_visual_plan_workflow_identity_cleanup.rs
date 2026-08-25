//! Removes the unused workflow identity from persistent visual plans.

use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N chapter visual plan workflow identity cleanup only supports PostgreSQL".into(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE chapter_visual_plan DROP COLUMN IF EXISTS workflow_run_id",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "chapter visual plan workflow identity cleanup is not reversible".into(),
        ))
    }
}
