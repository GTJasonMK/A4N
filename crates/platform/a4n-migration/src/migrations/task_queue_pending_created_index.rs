use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const CREATE_INDEX_SQL: &str = "CREATE INDEX IF NOT EXISTS idx_task_queue_pending_created ON task_queue (created_at DESC) WHERE status = 'pending'";
const DROP_INDEX_SQL: &str = "DROP INDEX IF EXISTS idx_task_queue_pending_created";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Pending task queue created index requires PostgreSQL".into(),
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
                "Pending task queue created index requires PostgreSQL".into(),
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
    fn creates_partial_created_index_for_pending_tasks() {
        assert!(CREATE_INDEX_SQL.contains("idx_task_queue_pending_created"));
        assert!(CREATE_INDEX_SQL.contains("task_queue (created_at DESC)"));
        assert!(CREATE_INDEX_SQL.contains("WHERE status = 'pending'"));
    }
}
