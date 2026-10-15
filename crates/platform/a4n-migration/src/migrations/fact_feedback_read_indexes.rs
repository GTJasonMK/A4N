//! Indexes for the bounded fact feedback summary read.

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Fact feedback read indexes only support PostgreSQL".into(),
            ));
        }
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "CREATE INDEX IF NOT EXISTS idx_material_fact_change_summary_latest
                 ON material_fact_change (project_id, id DESC)
                 WHERE material_kind <> 'chapter_feedback'",
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Fact feedback read indexes only support PostgreSQL".into(),
            ));
        }
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "DROP INDEX IF EXISTS idx_material_fact_change_summary_latest",
            ))
            .await?;
        Ok(())
    }
}
