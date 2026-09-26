use super::super::status::update_generation_run_record;
use super::creation::record_started_generation_run_with_checkpoint;
use crate::generation::{
    GenerationDbError, GenerationRunCheckpointContext, RUN_STATUS_COMPLETED,
    RecordCompletedGenerationRunWithCheckpointInput, RecordStartedGenerationRunInput,
};
use a4n_db_models::models::generation::generation_run;
use a4n_domain::generation::RecordCompletedGenerationRunInput;
use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction, TransactionTrait};

pub async fn record_completed_generation_run(
    db: &DatabaseConnection,
    input: RecordCompletedGenerationRunInput,
) -> Result<generation_run::Model, GenerationDbError> {
    record_completed_run_with_checkpoint_transaction(
        db,
        RecordCompletedGenerationRunWithCheckpointInput {
            run: input,
            checkpoint: GenerationRunCheckpointContext::default(),
        },
    )
    .await
}

pub async fn record_completed_generation_run_in_transaction(
    db: &DatabaseTransaction,
    input: RecordCompletedGenerationRunInput,
) -> Result<generation_run::Model, GenerationDbError> {
    record_completed_generation_run_with_checkpoint_in(
        db,
        input,
        GenerationRunCheckpointContext::default(),
    )
    .await
}

pub(crate) async fn record_completed_generation_run_with_checkpoint_in<C>(
    db: &C,
    input: RecordCompletedGenerationRunInput,
    checkpoint: GenerationRunCheckpointContext,
) -> Result<generation_run::Model, GenerationDbError>
where
    C: ConnectionTrait,
{
    let response_summary_json = input.response_summary_json.clone();
    let run = record_started_generation_run_with_checkpoint(
        db,
        completed_run_to_started_run(input),
        checkpoint,
    )
    .await?;
    update_generation_run_record(db, run, RUN_STATUS_COMPLETED, response_summary_json, None).await
}

pub async fn record_completed_run_with_checkpoint_transaction(
    db: &DatabaseConnection,
    input: RecordCompletedGenerationRunWithCheckpointInput,
) -> Result<generation_run::Model, GenerationDbError> {
    let tx = db.begin().await?;
    let run = record_completed_generation_run_with_checkpoint_in(&tx, input.run, input.checkpoint)
        .await?;
    tx.commit().await?;
    Ok(run)
}

fn completed_run_to_started_run(
    input: RecordCompletedGenerationRunInput,
) -> RecordStartedGenerationRunInput {
    RecordStartedGenerationRunInput {
        project_id: input.project_id,
        workflow: input.workflow,
        step_key: input.step_key,
        title: input.title,
        model_kind: input.model_kind,
        provider: input.provider,
        model: input.model,
        prompt_template_names_json: input.prompt_template_names_json,
        prompt_template_hashes_json: input.prompt_template_hashes_json,
        rendered_messages_json: input.rendered_messages_json,
        rendered_prompts_json: input.rendered_prompts_json,
        input_variables_json: input.input_variables_json,
        source_refs_json: input.source_refs_json,
        media_refs_json: input.media_refs_json,
        request_json: input.request_json,
        targets: input.targets,
    }
}
