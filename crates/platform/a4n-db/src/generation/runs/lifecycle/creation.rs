use super::super::targets::insert_generation_run_targets;
use super::checkpoint::normalize_checkpoint_context;
use super::record::{insert_generation_run, root_project_id_for_project};
use crate::generation::{
    CreateGenerationRunInput, GenerationDbError, GenerationRunCheckpointContext,
    RecordStartedGenerationRunInput,
};
use a4n_db_models::models::generation::generation_run;
use sea_orm::ConnectionTrait;

pub(crate) async fn record_started_generation_run<C>(
    db: &C,
    input: RecordStartedGenerationRunInput,
) -> Result<generation_run::Model, GenerationDbError>
where
    C: ConnectionTrait,
{
    record_started_generation_run_with_checkpoint(
        db,
        input,
        GenerationRunCheckpointContext::default(),
    )
    .await
}

pub(super) async fn record_started_generation_run_with_checkpoint<C>(
    db: &C,
    input: RecordStartedGenerationRunInput,
    checkpoint: GenerationRunCheckpointContext,
) -> Result<generation_run::Model, GenerationDbError>
where
    C: ConnectionTrait,
{
    if input.targets.is_empty() {
        return Err(GenerationDbError::InvalidArgument(
            "生成运行记录必须绑定至少一个目标资产".to_string(),
        ));
    }
    let root_project_id = root_project_id_for_project(db, input.project_id).await?;
    let checkpoint = normalize_checkpoint_context(checkpoint)?;
    let targets = input.targets;
    let run = insert_generation_run(
        db,
        CreateGenerationRunInput {
            project_id: input.project_id,
            root_project_id,
            workflow: input.workflow,
            step_key: input.step_key,
            title: input.title,
            model_kind: input.model_kind,
            provider: input.provider,
            model: input.model,
            checkpoint,
            prompt_template_names_json: input.prompt_template_names_json,
            prompt_template_hashes_json: input.prompt_template_hashes_json,
            rendered_messages_json: input.rendered_messages_json,
            rendered_prompts_json: input.rendered_prompts_json,
            input_variables_json: input.input_variables_json,
            source_refs_json: input.source_refs_json,
            media_refs_json: input.media_refs_json,
            request_json: input.request_json,
        },
    )
    .await?;
    insert_generation_run_targets(db, run.id, targets).await?;
    Ok(run)
}
