use sea_orm::{DbBackend, sea_query::IndexCreateStatement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

fn project_changed_index() -> IndexCreateStatement {
    Index::create()
        .name("idx_project_status_history_project_changed")
        .table(Alias::new("project_status_history"))
        .col(Alias::new("project_id"))
        .col((Alias::new("changed_at"), IndexOrder::Desc))
        .col((Alias::new("id"), IndexOrder::Desc))
        .if_not_exists()
        .to_owned()
}

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Project status history project changed index requires PostgreSQL".into(),
            ));
        }
        manager.create_index(project_changed_index()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Project status history project changed index requires PostgreSQL".into(),
            ));
        }
        manager
            .drop_index(
                Index::drop()
                    .name("idx_project_status_history_project_changed")
                    .table(Alias::new("project_status_history"))
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
    fn creates_project_changed_order_index_for_status_history() {
        let statement = project_changed_index().to_string(PostgresQueryBuilder);

        assert!(statement.contains("idx_project_status_history_project_changed"));
        assert!(statement.contains("project_status_history"));
        assert!(statement.contains("project_id"));
        assert!(statement.contains("changed_at"));
    }
}
