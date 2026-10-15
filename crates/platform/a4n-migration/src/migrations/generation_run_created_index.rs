use sea_orm::{DbBackend, sea_query::IndexCreateStatement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

fn created_index() -> IndexCreateStatement {
    Index::create()
        .name("idx_generation_run_created")
        .table(Alias::new("generation_run"))
        .col((Alias::new("created_at"), IndexOrder::Desc))
        .if_not_exists()
        .to_owned()
}

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Generation run created index requires PostgreSQL".into(),
            ));
        }
        manager.create_index(created_index()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Generation run created index requires PostgreSQL".into(),
            ));
        }
        manager
            .drop_index(
                Index::drop()
                    .name("idx_generation_run_created")
                    .table(Alias::new("generation_run"))
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
    fn creates_created_at_index_for_generation_run() {
        let statement = created_index().to_string(PostgresQueryBuilder);

        assert!(statement.contains("idx_generation_run_created"));
        assert!(statement.contains("generation_run"));
        assert!(statement.contains("created_at"));
    }
}
