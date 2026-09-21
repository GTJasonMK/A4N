//! Account-level opt-in for GUI-context Page CLI sessions.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N Page CLI migration only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                r#"ALTER TABLE "user" ADD COLUMN IF NOT EXISTS page_cli_enabled BOOLEAN NOT NULL DEFAULT false"#,
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N Page CLI migration only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                r#"ALTER TABLE "user" DROP COLUMN IF EXISTS page_cli_enabled"#,
            ))
            .await?;
        Ok(())
    }
}
