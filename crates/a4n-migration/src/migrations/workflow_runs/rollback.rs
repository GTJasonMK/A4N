use a4n_core::models::{
    generation::{generation_run, workflow_checkpoint, workflow_progress_event, workflow_run},
    task::task_queue,
};
use sea_orm::{ConnectionTrait, EntityName};
use sea_orm_migration::prelude::*;

pub(super) async fn apply(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
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
