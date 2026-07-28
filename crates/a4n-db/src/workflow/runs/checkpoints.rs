use super::*;

use super::queries::workflow_checkpoints;

pub async fn upsert_workflow_checkpoint(
    db: &DatabaseConnection,
    input: UpsertWorkflowCheckpointInput,
) -> Result<workflow_checkpoint::Model, WorkflowWriteError> {
    validate_positive_i64(input.workflow_run_id, "workflow_run_id")?;
    let step_key = required_text(input.step_key, "step_key")?;
    let status = required_text(input.status, "status")?;
    let now = Utc::now().naive_utc();
    let existing = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(input.workflow_run_id))
        .filter(workflow_checkpoint::Column::StepKey.eq(step_key.as_str()))
        .one(db)
        .await
        .map_err(WorkflowWriteError::Database)?;

    if let Some(existing) = existing {
        let mut active: workflow_checkpoint::ActiveModel = existing.into();
        active.step_order = Set(input.step_order);
        active.status = Set(status);
        active.input_fingerprint_sha256 =
            Set(normalize_optional_text(input.input_fingerprint_sha256));
        active.input_fingerprint_json = Set(input.input_fingerprint_json);
        active.checkpoint_json = Set(input.checkpoint_json);
        active.artifacts_json = Set(input.artifacts_json);
        active.generation_run_ids_json = Set(input.generation_run_ids_json);
        active.error_message = Set(normalize_optional_text(input.error_message));
        active.updated_at = Set(now);
        active.completed_at = Set(input.completed.then_some(now));
        return active
            .update(db)
            .await
            .map_err(WorkflowWriteError::Database);
    }

    workflow_checkpoint::ActiveModel {
        id: NotSet,
        workflow_run_id: Set(input.workflow_run_id),
        step_key: Set(step_key),
        step_order: Set(input.step_order),
        status: Set(status),
        input_fingerprint_sha256: Set(normalize_optional_text(input.input_fingerprint_sha256)),
        input_fingerprint_json: Set(input.input_fingerprint_json),
        checkpoint_json: Set(input.checkpoint_json),
        artifacts_json: Set(input.artifacts_json),
        generation_run_ids_json: Set(input.generation_run_ids_json),
        error_message: Set(normalize_optional_text(input.error_message)),
        created_at: Set(now),
        updated_at: Set(now),
        completed_at: Set(input.completed.then_some(now)),
    }
    .insert(db)
    .await
    .map_err(WorkflowWriteError::Database)
}

pub async fn workflow_checkpoint_models(
    db: &DatabaseConnection,
    workflow_run_id: i64,
) -> Result<Vec<workflow_checkpoint::Model>, WorkflowReadError> {
    workflow_checkpoints(db, workflow_run_id).await
}

pub async fn delete_workflow_checkpoints_by_keys(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    step_keys: &[&str],
) -> Result<u64, WorkflowWriteError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    let step_keys = step_keys
        .iter()
        .map(|key| required_text((*key).to_string(), "step_key"))
        .collect::<Result<Vec<_>, _>>()?;
    if step_keys.is_empty() {
        return Ok(0);
    }
    workflow_checkpoint::Entity::delete_many()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(workflow_checkpoint::Column::StepKey.is_in(step_keys))
        .exec(db)
        .await
        .map(|result| result.rows_affected)
        .map_err(WorkflowWriteError::Database)
}

pub async fn completed_workflow_checkpoint_by_fingerprint(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    step_key: &str,
    input_fingerprint_sha256: &str,
) -> Result<Option<workflow_checkpoint::Model>, WorkflowReadError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    let step_key = required_text(step_key.to_string(), "step_key")?;
    let input_fingerprint_sha256 = required_text(
        input_fingerprint_sha256.to_string(),
        "input_fingerprint_sha256",
    )?;
    workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(workflow_checkpoint::Column::StepKey.eq(step_key))
        .filter(workflow_checkpoint::Column::InputFingerprintSha256.eq(input_fingerprint_sha256))
        .filter(workflow_checkpoint::Column::Status.eq(WORKFLOW_STATUS_COMPLETED))
        .one(db)
        .await
        .map_err(WorkflowReadError::Database)
}

pub async fn workflow_checkpoint_model_by_step(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    step_key: &str,
) -> Result<Option<workflow_checkpoint::Model>, WorkflowReadError> {
    workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(workflow_checkpoint::Column::StepKey.eq(step_key))
        .one(db)
        .await
        .map_err(WorkflowReadError::Database)
}
