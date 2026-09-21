//! Remove the display-number identity constraint superseded by stable segment sync keys.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N drop_chapter_video_prompt_identity_unique migration only supports PostgreSQL"
                    .to_string(),
            ));
        }
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "DROP INDEX IF EXISTS idx_chapter_video_prompt_identity_unique".to_string(),
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "drop_chapter_video_prompt_identity_unique 不可逆：旧展示编号唯一键不会恢复"
                .to_string(),
        ))
    }
}
