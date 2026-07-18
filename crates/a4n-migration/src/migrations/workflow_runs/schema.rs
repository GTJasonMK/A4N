use a4n_core::models::{
    generation::{generation_run, workflow_checkpoint, workflow_progress_event, workflow_run},
    task::task_queue,
};
use sea_orm::{ConnectionTrait, DbBackend, EntityName, Schema};
use sea_orm_migration::prelude::*;

pub(super) async fn apply(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
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

    Ok(())
}
