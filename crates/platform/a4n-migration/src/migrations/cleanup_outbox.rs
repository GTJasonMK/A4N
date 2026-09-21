//! Durable project deletion cleanup outbox.

use a4n_db_models::models::task::cleanup_outbox;
use sea_orm::{DbBackend, EntityName, Schema};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

fn cleanup_outbox_table() -> TableCreateStatement {
    let schema = Schema::new(DbBackend::Postgres);
    let mut table = schema.create_table_from_entity(cleanup_outbox::Entity);
    table.if_not_exists();
    table
}

fn cleanup_outbox_dedupe_index() -> IndexCreateStatement {
    Index::create()
        .name("uidx_project_cleanup_outbox_dedupe")
        .table(Alias::new(cleanup_outbox::Entity.table_name()))
        .col(cleanup_outbox::Column::DedupeKey)
        .unique()
        .if_not_exists()
        .to_owned()
}

fn cleanup_outbox_claim_index() -> IndexCreateStatement {
    Index::create()
        .name("idx_project_cleanup_outbox_claim")
        .table(Alias::new(cleanup_outbox::Entity.table_name()))
        .col(cleanup_outbox::Column::Status)
        .col(cleanup_outbox::Column::NextAttemptAt)
        .col(cleanup_outbox::Column::LeaseExpiresAt)
        .if_not_exists()
        .to_owned()
}

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N cleanup outbox migration only supports PostgreSQL".to_string(),
            ));
        }
        manager.create_table(cleanup_outbox_table()).await?;
        manager.create_index(cleanup_outbox_dedupe_index()).await?;
        manager.create_index(cleanup_outbox_claim_index()).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new(cleanup_outbox::Entity.table_name()))
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm_migration::prelude::PostgresQueryBuilder;

    #[test]
    fn cleanup_outbox_uses_the_project_scoped_table_name() {
        assert_eq!(
            cleanup_outbox::Entity.table_name(),
            "project_cleanup_outbox",
            "the Rust module name must not be used as the PostgreSQL relation name"
        );
    }

    #[test]
    fn cleanup_outbox_statements_target_the_project_scoped_relation() {
        let statements = [
            cleanup_outbox_table().to_string(PostgresQueryBuilder),
            cleanup_outbox_dedupe_index().to_string(PostgresQueryBuilder),
            cleanup_outbox_claim_index().to_string(PostgresQueryBuilder),
        ];

        for statement in statements {
            assert!(
                statement.contains("\"project_cleanup_outbox\""),
                "cleanup outbox SQL must target project_cleanup_outbox: {statement}"
            );
            assert!(
                !statement.contains("ON \"cleanup_outbox\"")
                    && !statement.contains("TABLE \"cleanup_outbox\""),
                "cleanup outbox SQL must not target the migration/module name: {statement}"
            );
        }
    }
}
