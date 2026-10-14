use sea_orm::{DbBackend, sea_query::IndexCreateStatement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

fn project_created_index() -> IndexCreateStatement {
    Index::create()
        .name("idx_task_queue_project_created")
        .table(Alias::new("task_queue"))
        .col(Alias::new("project_id"))
        .col((Alias::new("created_at"), IndexOrder::Desc))
        .if_not_exists()
        .to_owned()
}

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Task queue project created index requires PostgreSQL".into(),
            ));
        }
        manager.create_index(project_created_index()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Task queue project created index requires PostgreSQL".into(),
            ));
        }
        manager
            .drop_index(
                Index::drop()
                    .name("idx_task_queue_project_created")
                    .table(Alias::new("task_queue"))
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use sea_orm_migration::prelude::PostgresQueryBuilder;

    use super::*;

    #[test]
    fn creates_project_created_order_index_for_task_history() {
        let statement = project_created_index().to_string(PostgresQueryBuilder);

        assert!(statement.contains("idx_task_queue_project_created"));
        assert!(statement.contains("task_queue"));
        assert!(statement.contains("project_id"));
        assert!(statement.contains("created_at"));
    }
}
