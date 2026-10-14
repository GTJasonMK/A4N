use sea_orm::{DbBackend, sea_query::IndexCreateStatement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

fn project_updated_index() -> IndexCreateStatement {
    Index::create()
        .name("idx_workflow_run_project_updated")
        .table(Alias::new("workflow_run"))
        .col(Alias::new("project_id"))
        .col((Alias::new("updated_at"), IndexOrder::Desc))
        .col((Alias::new("id"), IndexOrder::Desc))
        .if_not_exists()
        .to_owned()
}

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Workflow run project updated index requires PostgreSQL".into(),
            ));
        }
        manager.create_index(project_updated_index()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Workflow run project updated index requires PostgreSQL".into(),
            ));
        }
        manager
            .drop_index(
                Index::drop()
                    .name("idx_workflow_run_project_updated")
                    .table(Alias::new("workflow_run"))
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
    fn creates_project_updated_order_index_for_workflow_runs() {
        let statement = project_updated_index().to_string(PostgresQueryBuilder);

        assert!(statement.contains("idx_workflow_run_project_updated"));
        assert!(statement.contains("workflow_run"));
        assert!(statement.contains("project_id"));
        assert!(statement.contains("updated_at"));
        assert!(statement.contains("id"));
    }
}
