//! Galgame script tables.

use a4n_db_models::models::galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script};
use sea_orm::{DbBackend, EntityName, Schema};
use sea_orm_migration::prelude::*;

macro_rules! create_entity_table {
    ($manager:expr, $schema:expr, $entity:expr) => {{
        let mut table = $schema.create_table_from_entity($entity);
        table.if_not_exists();
        $manager.create_table(table).await?;
    }};
}

macro_rules! create_entity_index {
    ($manager:expr, $name:expr, $entity:expr, [$($column:expr),+], unique) => {{
        let mut index = Index::create();
        index
            .name($name)
            .table(Alias::new($entity.table_name()))
            .if_not_exists()
            .unique();
        $(index.col($column);)+
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

macro_rules! drop_entity_table {
    ($manager:expr, $entity:expr) => {{
        $manager
            .drop_table(
                Table::drop()
                    .table(Alias::new($entity.table_name()))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
    }};
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N galgame script migration only supports PostgreSQL".to_string(),
            ));
        }
        let schema = Schema::new(DbBackend::Postgres);

        create_entity_table!(manager, schema, galgame_script::Entity);
        create_entity_table!(manager, schema, galgame_scene::Entity);
        create_entity_table!(manager, schema, galgame_line::Entity);
        create_entity_table!(manager, schema, galgame_choice::Entity);

        create_entity_index!(
            manager,
            "uidx_galgame_script_project",
            galgame_script::Entity,
            [galgame_script::Column::ProjectId],
            unique
        );
        create_entity_index!(
            manager,
            "idx_galgame_script_root_snapshot",
            galgame_script::Entity,
            [
                galgame_script::Column::RootProjectId,
                galgame_script::Column::SourceSnapshotId
            ]
        );
        create_entity_index!(
            manager,
            "uidx_galgame_scene_script_index",
            galgame_scene::Entity,
            [
                galgame_scene::Column::ScriptId,
                galgame_scene::Column::SceneIndex
            ],
            unique
        );
        create_entity_index!(
            manager,
            "idx_galgame_scene_snapshot_chapter",
            galgame_scene::Entity,
            [galgame_scene::Column::SourceSnapshotChapterId]
        );
        create_entity_index!(
            manager,
            "idx_galgame_line_scene_index",
            galgame_line::Entity,
            [
                galgame_line::Column::SceneId,
                galgame_line::Column::LineIndex
            ]
        );
        create_entity_index!(
            manager,
            "idx_galgame_line_text_anchor",
            galgame_line::Entity,
            [galgame_line::Column::TextAnchorId]
        );
        create_entity_index!(
            manager,
            "uidx_galgame_choice_line_index",
            galgame_choice::Entity,
            [
                galgame_choice::Column::LineId,
                galgame_choice::Column::OptionIndex
            ],
            unique
        );

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N galgame script migration only supports PostgreSQL".to_string(),
            ));
        }
        drop_entity_table!(manager, galgame_choice::Entity);
        drop_entity_table!(manager, galgame_line::Entity);
        drop_entity_table!(manager, galgame_scene::Entity);
        drop_entity_table!(manager, galgame_script::Entity);
        Ok(())
    }
}
