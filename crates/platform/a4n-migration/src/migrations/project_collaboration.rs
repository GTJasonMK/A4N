//! Project collaboration members and invitations.

use a4n_db_models::models::project::{project_invite, project_member};
use sea_orm::{ConnectionTrait, DbBackend, EntityName, Schema, Statement};
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

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N project collaboration migration only supports PostgreSQL".to_string(),
            ));
        }

        let schema = Schema::new(DbBackend::Postgres);

        let mut member_table = schema.create_table_from_entity(project_member::Entity);
        member_table.if_not_exists();
        manager.create_table(member_table).await?;

        let mut invite_table = schema.create_table_from_entity(project_invite::Entity);
        invite_table.if_not_exists();
        manager.create_table(invite_table).await?;

        create_entity_index!(
            manager,
            "idx_project_member_project_user_unique",
            project_member::Entity,
            [
                project_member::Column::ProjectId,
                project_member::Column::UserId
            ],
            unique
        );
        create_entity_index!(
            manager,
            "idx_project_member_user",
            project_member::Entity,
            [project_member::Column::UserId]
        );
        create_entity_index!(
            manager,
            "idx_project_invite_project",
            project_invite::Entity,
            [project_invite::Column::ProjectId]
        );
        create_entity_index!(
            manager,
            "idx_project_invite_invitee",
            project_invite::Entity,
            [project_invite::Column::InviteeUserId]
        );
        create_entity_index!(
            manager,
            "idx_project_invite_status",
            project_invite::Entity,
            [project_invite::Column::Status]
        );

        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_project_invite_pending_unique \
                 ON project_invite(project_id, invitee_user_id) WHERE status = 'pending'"
                    .to_string(),
            ))
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N project collaboration migration only supports PostgreSQL".to_string(),
            ));
        }

        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new(project_invite::Entity.table_name()))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new(project_member::Entity.table_name()))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
