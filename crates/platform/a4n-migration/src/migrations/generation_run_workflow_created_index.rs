use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const CREATE_INDEX_SQL: &str = "CREATE INDEX IF NOT EXISTS idx_generation_run_workflow_created ON generation_run (workflow_run_id, created_at ASC) WHERE workflow_run_id IS NOT NULL";
const DROP_INDEX_SQL: &str = "DROP INDEX IF EXISTS idx_generation_run_workflow_created";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Workflow generation run index requires PostgreSQL".into(),
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
                "Workflow generation run index requires PostgreSQL".into(),
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
    fn creates_partial_workflow_run_created_index() {
        assert!(CREATE_INDEX_SQL.contains("idx_generation_run_workflow_created"));
        assert!(CREATE_INDEX_SQL.contains("generation_run (workflow_run_id, created_at ASC)"));
        assert!(CREATE_INDEX_SQL.contains("WHERE workflow_run_id IS NOT NULL"));
    }
}
