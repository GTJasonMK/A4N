use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const CREATE_INDEX_SQL: &str = "CREATE INDEX IF NOT EXISTS idx_model_usage_record_root_started ON model_usage_record (root_project_id, started_at DESC) WHERE root_project_id IS NOT NULL";
const DROP_INDEX_SQL: &str = "DROP INDEX IF EXISTS idx_model_usage_record_root_started";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Model usage root project index requires PostgreSQL".into(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(CREATE_INDEX_SQL)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Model usage root project index requires PostgreSQL".into(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(DROP_INDEX_SQL)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::CREATE_INDEX_SQL;

    #[test]
    fn creates_partial_root_project_started_index() {
        assert!(CREATE_INDEX_SQL.contains("idx_model_usage_record_root_started"));
        assert!(CREATE_INDEX_SQL.contains("model_usage_record (root_project_id, started_at DESC)"));
        assert!(CREATE_INDEX_SQL.contains("WHERE root_project_id IS NOT NULL"));
    }
}
