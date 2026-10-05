//! Persistent Galgame runtime save slots.

use a4n_db_models::models::galgame::galgame_story_runtime_state;
use sea_orm::Schema;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let schema = Schema::new(manager.get_database_backend());
        manager
            .create_table(
                schema
                    .create_table_from_entity(galgame_story_runtime_state::Entity)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("uidx_galgame_story_runtime_user_project_slot")
                    .table(galgame_story_runtime_state::Entity)
                    .col(galgame_story_runtime_state::Column::UserId)
                    .col(galgame_story_runtime_state::Column::ProjectId)
                    .col(galgame_story_runtime_state::Column::SaveSlot)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(galgame_story_runtime_state::Entity)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
