//! One-time registration invite codes.

use a4n_core::models::user::invite_code;
use sea_orm::{DbBackend, EntityName, Schema};
use sea_orm_migration::prelude::*;

macro_rules! create_entity_index {
    ($manager:expr, $name:expr, $entity:expr, [$($column:expr),+], unique) => {{
        let mut index = Index::create();
        index
            .name($name)
            .table(Alias::new($entity.table_name()))
            .if_not_exists();
        $(index.col($column);)+
        index.unique();
        $manager.create_index(index.to_owned()).await?;
    }};
    ($manager:expr, $name:expr, $entity:expr, [$($column:expr),+]) => {{
        let mut index = Index::create();
        index
            .name($name)
            .table(Alias::new($entity.table_name()))
            .if_not_exists();
        $(index.col($column);)+
        $manager.create_index(index.to_owned()).await?;
    }};
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N invite code migration only supports PostgreSQL".to_string(),
            ));
        }

        let schema = Schema::new(DbBackend::Postgres);
        let mut table = schema.create_table_from_entity(invite_code::Entity);
        table.if_not_exists();
        manager.create_table(table).await?;

        create_entity_index!(
            manager,
            "idx_invite_code_code_unique",
            invite_code::Entity,
            [invite_code::Column::Code],
            unique
        );
        create_entity_index!(
            manager,
            "idx_invite_code_created_by_user",
            invite_code::Entity,
            [invite_code::Column::CreatedByUserId]
        );
        create_entity_index!(
            manager,
            "idx_invite_code_used_by_user",
            invite_code::Entity,
            [invite_code::Column::UsedByUserId]
        );

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N invite code migration only supports PostgreSQL".to_string(),
            ));
        }

        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new(invite_code::Entity.table_name()))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
