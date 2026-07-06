//! Persistent workflow runs and checkpoints.

use a4n_core::models::{
    generation::{generation_run, workflow_checkpoint, workflow_progress_event, workflow_run},
    task::task_queue,
};
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
            .if_not_exists()
            .unique();
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
                "A4N workflow run migration only supports PostgreSQL".to_string(),
            ));
        }

        let schema = Schema::new(DbBackend::Postgres);

        let mut run_table = schema.create_table_from_entity(workflow_run::Entity);
        run_table.if_not_exists();
        manager.create_table(run_table).await?;

        let mut checkpoint_table = schema.create_table_from_entity(workflow_checkpoint::Entity);
        checkpoint_table.if_not_exists();
        manager.create_table(checkpoint_table).await?;
        manager
            .get_connection()
            .execute_unprepared(&format!(
                "ALTER TABLE {} ADD COLUMN IF NOT EXISTS input_fingerprint_sha256 TEXT",
                workflow_checkpoint::Entity.table_name()
            ))
            .await?;

        let mut event_table = schema.create_table_from_entity(workflow_progress_event::Entity);
        event_table.if_not_exists();
        manager.create_table(event_table).await?;

        manager
            .get_connection()
            .execute_unprepared(&format!(
                "ALTER TABLE {} ADD COLUMN IF NOT EXISTS workflow_run_id BIGINT",
                generation_run::Entity.table_name()
            ))
            .await?;
        manager
            .get_connection()
            .execute_unprepared(&format!(
                "ALTER TABLE {} ADD COLUMN IF NOT EXISTS call_key TEXT",
                generation_run::Entity.table_name()
            ))
            .await?;
        manager
            .get_connection()
            .execute_unprepared(&format!(
                "ALTER TABLE {} ADD COLUMN IF NOT EXISTS input_fingerprint_sha256 TEXT",
                generation_run::Entity.table_name()
            ))
            .await?;
        manager
            .get_connection()
            .execute_unprepared(&format!(
                "ALTER TABLE {} ADD COLUMN IF NOT EXISTS input_fingerprint_json JSONB",
                generation_run::Entity.table_name()
            ))
            .await?;
        manager
            .get_connection()
            .execute_unprepared(&format!(
                "ALTER TABLE {} ADD COLUMN IF NOT EXISTS raw_response_json JSONB",
                generation_run::Entity.table_name()
            ))
            .await?;
        manager
            .get_connection()
            .execute_unprepared(&format!(
                "ALTER TABLE {} ADD COLUMN IF NOT EXISTS parsed_output_json JSONB",
                generation_run::Entity.table_name()
            ))
            .await?;
        manager
            .get_connection()
            .execute_unprepared(&format!(
                "ALTER TABLE {} ADD COLUMN IF NOT EXISTS validation_json JSONB",
                generation_run::Entity.table_name()
            ))
            .await?;
        manager
            .get_connection()
            .execute_unprepared(&format!(
                "ALTER TABLE {} ADD COLUMN IF NOT EXISTS artifact_refs_json JSONB",
                generation_run::Entity.table_name()
            ))
            .await?;
        manager
            .get_connection()
            .execute_unprepared(&format!(
                "ALTER TABLE {} ADD COLUMN IF NOT EXISTS attempt INTEGER NOT NULL DEFAULT 1",
                generation_run::Entity.table_name()
            ))
            .await?;
        manager
            .get_connection()
            .execute_unprepared(&format!(
                "ALTER TABLE {} ADD COLUMN IF NOT EXISTS workflow_run_id BIGINT",
                task_queue::Entity.table_name()
            ))
            .await?;

        create_entity_index!(
            manager,
            "idx_workflow_run_business_key",
            workflow_run::Entity,
            [workflow_run::Column::BusinessKey]
        );
        create_entity_index!(
            manager,
            "idx_workflow_run_project_status",
            workflow_run::Entity,
            [
                workflow_run::Column::ProjectId,
                workflow_run::Column::Status
            ]
        );
        create_entity_index!(
            manager,
            "idx_workflow_run_kind_updated",
            workflow_run::Entity,
            [
                workflow_run::Column::WorkflowKind,
                workflow_run::Column::UpdatedAt
            ]
        );
        manager
            .get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS idx_workflow_checkpoint_run_step")
            .await?;
        create_unique_entity_index!(
            manager,
            "uidx_workflow_checkpoint_run_step",
            workflow_checkpoint::Entity,
            [
                workflow_checkpoint::Column::WorkflowRunId,
                workflow_checkpoint::Column::StepKey
            ]
        );
        create_entity_index!(
            manager,
            "idx_workflow_checkpoint_run_step_fingerprint",
            workflow_checkpoint::Entity,
            [
                workflow_checkpoint::Column::WorkflowRunId,
                workflow_checkpoint::Column::StepKey,
                workflow_checkpoint::Column::InputFingerprintSha256
            ]
        );
        create_entity_index!(
            manager,
            "idx_workflow_progress_event_run_index",
            workflow_progress_event::Entity,
            [
                workflow_progress_event::Column::WorkflowRunId,
                workflow_progress_event::Column::EventIndex
            ]
        );
        create_entity_index!(
            manager,
            "idx_generation_run_workflow_run",
            generation_run::Entity,
            [generation_run::Column::WorkflowRunId]
        );
        create_entity_index!(
            manager,
            "idx_generation_run_workflow_step_call",
            generation_run::Entity,
            [
                generation_run::Column::WorkflowRunId,
                generation_run::Column::StepKey,
                generation_run::Column::CallKey
            ]
        );
        create_entity_index!(
            manager,
            "idx_generation_run_workflow_call_fingerprint",
            generation_run::Entity,
            [
                generation_run::Column::WorkflowRunId,
                generation_run::Column::StepKey,
                generation_run::Column::CallKey,
                generation_run::Column::InputFingerprintSha256
            ]
        );
        create_entity_index!(
            manager,
            "idx_generation_run_workflow_status",
            generation_run::Entity,
            [
                generation_run::Column::WorkflowRunId,
                generation_run::Column::Status
            ]
        );
        create_entity_index!(
            manager,
            "idx_task_queue_workflow_run",
            task_queue::Entity,
            [task_queue::Column::WorkflowRunId]
        );

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N workflow run migration only supports PostgreSQL".to_string(),
            ));
        }

        let generation_table = generation_run::Entity.table_name();
        let task_queue_table = task_queue::Entity.table_name();
        for index_name in [
            "idx_generation_run_workflow_status",
            "idx_generation_run_workflow_call_fingerprint",
            "idx_generation_run_workflow_step_call",
            "idx_generation_run_workflow_run",
            "idx_task_queue_workflow_run",
        ] {
            manager
                .get_connection()
                .execute_unprepared(&format!("DROP INDEX IF EXISTS {index_name}"))
                .await?;
        }
        manager
            .get_connection()
            .execute_unprepared(&format!(
                r#"
                ALTER TABLE {generation_table}
                    DROP COLUMN IF EXISTS workflow_run_id,
                    DROP COLUMN IF EXISTS call_key,
                    DROP COLUMN IF EXISTS input_fingerprint_sha256,
                    DROP COLUMN IF EXISTS input_fingerprint_json,
                    DROP COLUMN IF EXISTS raw_response_json,
                    DROP COLUMN IF EXISTS parsed_output_json,
                    DROP COLUMN IF EXISTS validation_json,
                    DROP COLUMN IF EXISTS artifact_refs_json,
                    DROP COLUMN IF EXISTS attempt;
                "#
            ))
            .await?;
        manager
            .get_connection()
            .execute_unprepared(&format!(
                "ALTER TABLE {task_queue_table} DROP COLUMN IF EXISTS workflow_run_id"
            ))
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new(workflow_progress_event::Entity.table_name()))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new(workflow_checkpoint::Entity.table_name()))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new(workflow_run::Entity.table_name()))
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
