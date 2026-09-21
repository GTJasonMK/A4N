//! Persistent generation run provenance.

use a4n_db_models::models::generation::{generation_run, generation_run_target};
use sea_orm::{DbBackend, EntityName, Schema};
use sea_orm_migration::prelude::*;

macro_rules! create_entity_index {
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
                "A4N generation run migration only supports PostgreSQL".to_string(),
            ));
        }

        let schema = Schema::new(DbBackend::Postgres);
        let mut run_table = schema.create_table_from_entity(generation_run::Entity);
        run_table.if_not_exists();
        manager.create_table(run_table).await?;

        let mut target_table = schema.create_table_from_entity(generation_run_target::Entity);
        target_table.if_not_exists();
        manager.create_table(target_table).await?;

        create_entity_index!(
            manager,
            "idx_generation_run_project_created",
            generation_run::Entity,
            [
                generation_run::Column::ProjectId,
                generation_run::Column::CreatedAt
            ]
        );
        create_entity_index!(
            manager,
            "idx_generation_run_workflow_step",
            generation_run::Entity,
            [
                generation_run::Column::Workflow,
                generation_run::Column::StepKey
            ]
        );
        create_entity_index!(
            manager,
            "idx_generation_run_target_lookup",
            generation_run_target::Entity,
            [
                generation_run_target::Column::TargetTable,
                generation_run_target::Column::TargetId
            ]
        );
        create_entity_index!(
            manager,
            "idx_generation_run_target_run",
            generation_run_target::Entity,
            [generation_run_target::Column::GenerationRunId]
        );

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N generation run migration only supports PostgreSQL".to_string(),
            ));
        }

        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new(generation_run_target::Entity.table_name()))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new(generation_run::Entity.table_name()))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
