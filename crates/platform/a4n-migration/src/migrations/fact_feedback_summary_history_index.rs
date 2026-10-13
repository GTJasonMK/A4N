use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Fact feedback summary history index requires PostgreSQL".into(),
            ));
        }
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "CREATE INDEX IF NOT EXISTS idx_material_fact_change_summary_history
                    ON material_fact_change (project_id, id DESC)
                    WHERE material_kind <> 'chapter_feedback'
                      AND before_json::text <> after_json::text",
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Fact feedback summary history index requires PostgreSQL".into(),
            ));
        }
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "DROP INDEX IF EXISTS idx_material_fact_change_summary_history",
            ))
            .await?;
        Ok(())
    }
}
