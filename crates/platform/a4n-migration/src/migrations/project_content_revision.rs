//! Durable project content revision for cross-window reset synchronization.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

const ADD_CONTENT_REVISION: &str =
    "ALTER TABLE novel_project ADD COLUMN IF NOT EXISTS content_revision BIGINT NOT NULL DEFAULT 0";
const DROP_CONTENT_REVISION: &str =
    "ALTER TABLE novel_project DROP COLUMN IF EXISTS content_revision";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N project content revision migration only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                ADD_CONTENT_REVISION.to_string(),
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N project content revision migration only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                DROP_CONTENT_REVISION.to_string(),
            ))
            .await?;
        Ok(())
    }
}
