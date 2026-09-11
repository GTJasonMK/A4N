use a4n_db_models::models::{
    generation::{generation_run, workflow_checkpoint, workflow_progress_event, workflow_run},
    task::task_queue,
};
use sea_orm::{ConnectionTrait, EntityName};
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

pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
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
