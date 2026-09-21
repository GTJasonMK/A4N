//! Persistent LLM context packs and cache/cost traces.

use a4n_db_models::models::generation::{llm_call_trace, llm_context_pack};
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

macro_rules! create_unique_entity_index {
    ($manager:expr, $name:expr, $entity:expr, [$($column:expr),+]) => {{
        let mut index = Index::create();
        index
            .name($name)
            .table(Alias::new($entity.table_name()))
            .unique()
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
                "A4N LLM context tracking migration only supports PostgreSQL".to_string(),
            ));
        }

        let schema = Schema::new(DbBackend::Postgres);
        let mut context_pack_table = schema.create_table_from_entity(llm_context_pack::Entity);
        context_pack_table.if_not_exists();
        manager.create_table(context_pack_table).await?;

        let mut call_trace_table = schema.create_table_from_entity(llm_call_trace::Entity);
        call_trace_table.if_not_exists();
        manager.create_table(call_trace_table).await?;

        create_entity_index!(
            manager,
            "idx_llm_context_pack_lookup",
            llm_context_pack::Entity,
            [
                llm_context_pack::Column::ProjectId,
                llm_context_pack::Column::Workflow,
                llm_context_pack::Column::PackKind,
                llm_context_pack::Column::CompressionLevel
            ]
        );
        create_entity_index!(
            manager,
            "idx_llm_context_pack_hash",
            llm_context_pack::Entity,
            [
                llm_context_pack::Column::ContentSha256,
                llm_context_pack::Column::ConfigHash
            ]
        );
        create_unique_entity_index!(
            manager,
            "uidx_llm_context_pack_identity",
            llm_context_pack::Entity,
            [
                llm_context_pack::Column::ProjectId,
                llm_context_pack::Column::Workflow,
                llm_context_pack::Column::PackKind,
                llm_context_pack::Column::CompressionLevel,
                llm_context_pack::Column::ContentSha256,
                llm_context_pack::Column::ConfigHash
            ]
        );
        create_entity_index!(
            manager,
            "idx_llm_call_trace_project_created",
            llm_call_trace::Entity,
            [
                llm_call_trace::Column::ProjectId,
                llm_call_trace::Column::CreatedAt
            ]
        );
        create_entity_index!(
            manager,
            "idx_llm_call_trace_workflow_step",
            llm_call_trace::Entity,
            [
                llm_call_trace::Column::Workflow,
                llm_call_trace::Column::StepKey
            ]
        );
        create_entity_index!(
            manager,
            "idx_llm_call_trace_generation_run",
            llm_call_trace::Entity,
            [llm_call_trace::Column::GenerationRunId]
        );

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N LLM context tracking migration only supports PostgreSQL".to_string(),
            ));
        }

        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new(llm_call_trace::Entity.table_name()))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new(llm_context_pack::Entity.table_name()))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
