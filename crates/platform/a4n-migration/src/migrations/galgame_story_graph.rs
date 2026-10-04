//! Versioned Galgame story graph documents.

use a4n_db_models::models::galgame::galgame_story_graph;
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
                    .create_table_from_entity(galgame_story_graph::Entity)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("uidx_galgame_story_graph_project_revision")
                    .table(galgame_story_graph::Entity)
                    .col(galgame_story_graph::Column::ProjectId)
                    .col(galgame_story_graph::Column::Revision)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_galgame_story_graph_project_status")
                    .table(galgame_story_graph::Entity)
                    .col(galgame_story_graph::Column::ProjectId)
                    .col(galgame_story_graph::Column::Status)
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
                    .table(galgame_story_graph::Entity)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}
