use super::checkpoint::normalize_checkpoint_context;
use crate::generation::{
    CreateGenerationRunInput, GenerationDbError, RUN_STATUS_RUNNING, normalize_optional_owned_text,
    required_owned_text, validate_positive_id,
};
use a4n_db_models::models::{generation::generation_run, project::novel_project};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, QuerySelect, Set};

pub(super) async fn insert_generation_run<C>(
    db: &C,
    input: CreateGenerationRunInput,
) -> Result<generation_run::Model, GenerationDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(input.project_id, "project_id")?;
    validate_positive_id(input.root_project_id, "root_project_id")?;
    let workflow = required_owned_text(input.workflow, "workflow")?;
    let step_key = required_owned_text(input.step_key, "step_key")?;
    let title = required_owned_text(input.title, "title")?;
    let model_kind = required_owned_text(input.model_kind, "model_kind")?;
    let checkpoint = normalize_checkpoint_context(input.checkpoint)?;
    let now = Utc::now().naive_utc();

    generation_run::ActiveModel {
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        workflow: Set(workflow),
        step_key: Set(step_key),
        title: Set(title),
        model_kind: Set(model_kind),
        provider: Set(normalize_optional_owned_text(input.provider)),
        model: Set(normalize_optional_owned_text(input.model)),
        status: Set(RUN_STATUS_RUNNING.to_string()),
        workflow_run_id: Set(checkpoint.workflow_run_id),
        call_key: Set(checkpoint.call_key),
        input_fingerprint_sha256: Set(checkpoint.input_fingerprint_sha256),
        input_fingerprint_json: Set(checkpoint.input_fingerprint_json),
        raw_response_json: Set(checkpoint.raw_response_json),
        parsed_output_json: Set(checkpoint.parsed_output_json),
        validation_json: Set(checkpoint.validation_json),
        artifact_refs_json: Set(checkpoint.artifact_refs_json),
        attempt: Set(checkpoint.attempt.unwrap_or(1)),
        prompt_template_names_json: Set(input.prompt_template_names_json),
        prompt_template_hashes_json: Set(input.prompt_template_hashes_json),
        rendered_messages_json: Set(input.rendered_messages_json),
        rendered_prompts_json: Set(input.rendered_prompts_json),
        input_variables_json: Set(input.input_variables_json),
        source_refs_json: Set(input.source_refs_json),
        media_refs_json: Set(input.media_refs_json),
        request_json: Set(input.request_json),
        response_summary_json: Set(None),
        error_message: Set(None),
        created_at: Set(now),
        completed_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(GenerationDbError::Database)
}

pub(super) async fn root_project_id_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<i64, GenerationDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(project_id, "project_id")?;
    let project = novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::RootProjectId)
        .into_tuple::<(i64, Option<i64>)>()
        .one(db)
        .await?
        .ok_or_else(|| GenerationDbError::NotFound(format!("项目不存在: {project_id}")))?;
    Ok(project.1.unwrap_or(project.0))
}
