//! Host package state, lifecycle operations, and per-user display preferences.

use a4n_db_models::models::package::{
    host_package_operation, host_package_state, user_package_preference,
};
use sea_orm::{ConnectionTrait, DbBackend, EntityName, EntityTrait, Schema, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

fn table_from_entity<E: EntityName + EntityTrait>(entity: E) -> TableCreateStatement {
    Schema::new(DbBackend::Postgres)
        .create_table_from_entity(entity)
        .if_not_exists()
        .to_owned()
}

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N package lifecycle migration only supports PostgreSQL".to_string(),
            ));
        }

        manager
            .create_table(table_from_entity(host_package_state::Entity))
            .await?;
        manager
            .create_table(table_from_entity(host_package_operation::Entity))
            .await?;
        manager
            .create_table(table_from_entity(user_package_preference::Entity))
            .await?;

        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                r#"
                ALTER TABLE host_package_state
                    ADD CONSTRAINT host_package_state_installation_check
                    CHECK (installation_state IN ('installed', 'missing', 'incompatible', 'unknown')),
                    ADD CONSTRAINT host_package_state_host_state_check
                    CHECK (host_state IN ('enabled', 'disabled', 'enabling', 'disabling')),
                    ADD CONSTRAINT host_package_state_version_check
                    CHECK (status_version >= 0)
                "#,
            ))
            .await?;
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                r#"
                ALTER TABLE host_package_operation
                    ADD CONSTRAINT host_package_operation_action_check
                    CHECK (action IN ('enable', 'disable')),
                    ADD CONSTRAINT host_package_operation_state_check
                    CHECK (state IN ('pending', 'draining', 'waiting_external', 'failed', 'completed')),
                    ADD CONSTRAINT host_package_operation_version_check
                    CHECK (status_version >= 0)
                "#,
            ))
            .await?;
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                r#"CREATE INDEX IF NOT EXISTS idx_host_package_operation_package
                   ON host_package_operation (host_scope, package_id, updated_at DESC)"#,
            ))
            .await?;
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                r#"CREATE INDEX IF NOT EXISTS idx_user_package_preference_host
                   ON user_package_preference (host_scope, user_id)"#,
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new(user_package_preference::Entity.table_name()))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new(host_package_operation::Entity.table_name()))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new(host_package_state::Entity.table_name()))
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
    fn package_tables_have_stable_names() {
        assert_eq!(
            host_package_state::Entity.table_name(),
            "host_package_state"
        );
        assert_eq!(
            host_package_operation::Entity.table_name(),
            "host_package_operation"
        );
        assert_eq!(
            user_package_preference::Entity.table_name(),
            "user_package_preference"
        );
        let statement =
            table_from_entity(host_package_state::Entity).to_string(PostgresQueryBuilder);
        assert!(statement.contains("host_package_state"));
    }
}
