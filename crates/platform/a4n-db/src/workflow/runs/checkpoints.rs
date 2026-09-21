use super::*;

use super::queries::{locked_workflow_run_by_id, workflow_checkpoints};
use super::validation::ensure_workflow_write_open;
use sea_orm::{FromQueryResult, sea_query::extension::postgres::PgExpr};

pub async fn upsert_workflow_checkpoint(
    db: &DatabaseConnection,
    input: UpsertWorkflowCheckpointInput,
) -> Result<workflow_checkpoint::Model, WorkflowWriteError> {
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    let checkpoint = upsert_workflow_checkpoint_in_transaction(&tx, input).await?;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(checkpoint)
}

pub async fn upsert_workflow_checkpoint_in_transaction(
    tx: &sea_orm::DatabaseTransaction,
    input: UpsertWorkflowCheckpointInput,
) -> Result<workflow_checkpoint::Model, WorkflowWriteError> {
    validate_positive_i64(input.workflow_run_id, "workflow_run_id")?;
    let step_key = required_text(input.step_key, "step_key")?;
    let status = required_text(input.status, "status")?;
    let run = locked_workflow_run_by_id(tx, input.workflow_run_id).await?;
    ensure_workflow_write_open(&run.status, "写入工作流 checkpoint")?;
    let now = Utc::now().naive_utc();
    let existing = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(input.workflow_run_id))
        .filter(workflow_checkpoint::Column::StepKey.eq(step_key.as_str()))
        .one(tx)
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
        let updated = active
            .update(tx)
            .await
            .map_err(WorkflowWriteError::Database)?;
        return Ok(updated);
    }

    let inserted = workflow_checkpoint::ActiveModel {
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
    .insert(tx)
    .await
    .map_err(WorkflowWriteError::Database)?;
    Ok(inserted)
}

#[derive(Debug, FromQueryResult)]
struct WorkflowCheckpointArtifactState {
    parsed_output_json: Option<Value>,
    artifacts_json: Option<Value>,
}

pub async fn mutate_completed_workflow_checkpoint_artifacts_unless_step_exists<F>(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    step_key: &str,
    blocking_step_key: &str,
    mutate: F,
) -> Result<(), WorkflowWriteError>
where
    F: FnOnce(Option<&Value>, Option<&Value>) -> Result<Value, WorkflowWriteError> + Send,
{
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    let step_key = required_text(step_key.to_string(), "step_key")?;
    let blocking_step_key = required_text(blocking_step_key.to_string(), "blocking_step_key")?;
    if step_key == blocking_step_key {
        return Err(WorkflowWriteError::InvalidArgument(
            "step_key 与 blocking_step_key 不能相同".to_string(),
        ));
    }
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    let run = locked_workflow_run_by_id(&tx, workflow_run_id).await?;
    if !workflow_run_is_running_or_resumable(&run.status, run.can_resume) {
        return Err(WorkflowWriteError::StatusConflict {
            current: run.status,
            operation: "更新工作流 checkpoint 产物".to_string(),
        });
    }
    if workflow_checkpoint::Entity::find()
        .select_only()
        .column(workflow_checkpoint::Column::Id)
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(workflow_checkpoint::Column::StepKey.eq(blocking_step_key.as_str()))
        .into_tuple::<i64>()
        .one(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?
        .is_some()
    {
        return Err(WorkflowWriteError::StatusConflict {
            current: format!("checkpoint_exists:{blocking_step_key}"),
            operation: format!("更新 checkpoint {step_key} 产物"),
        });
    }
    let checkpoint = workflow_checkpoint::Entity::find()
        .select_only()
        .column_as(
            sea_orm::sea_query::Expr::col(workflow_checkpoint::Column::CheckpointJson)
                .get_json_field("parsed_output"),
            "parsed_output_json",
        )
        .column(workflow_checkpoint::Column::ArtifactsJson)
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(workflow_checkpoint::Column::StepKey.eq(step_key.as_str()))
        .filter(workflow_checkpoint::Column::Status.eq(WORKFLOW_STATUS_COMPLETED))
        .into_model::<WorkflowCheckpointArtifactState>()
        .one(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?
        .ok_or_else(|| {
            WorkflowWriteError::NotFound(format!(
                "工作流 {workflow_run_id} 不存在已完成 checkpoint {step_key}"
            ))
        })?;
    let artifacts_json = mutate(
        checkpoint.parsed_output_json.as_ref(),
        checkpoint.artifacts_json.as_ref(),
    )?;
    let now = Utc::now().naive_utc();
    let updated = workflow_checkpoint::Entity::update_many()
        .col_expr(
            workflow_checkpoint::Column::ArtifactsJson,
            sea_orm::sea_query::Expr::value(Some(artifacts_json)),
        )
        .col_expr(
            workflow_checkpoint::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(workflow_checkpoint::Column::StepKey.eq(step_key.as_str()))
        .exec(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?;
    if updated.rows_affected != 1 {
        return Err(WorkflowWriteError::NotFound(format!(
            "工作流 {workflow_run_id} 不存在 checkpoint {step_key}"
        )));
    }
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(())
}

pub async fn workflow_checkpoint_models(
    db: &DatabaseConnection,
    workflow_run_id: i64,
) -> Result<Vec<workflow_checkpoint::Model>, WorkflowReadError> {
    workflow_checkpoints(db, workflow_run_id).await
}

pub async fn workflow_checkpoint_models_for_runs(
    db: &DatabaseConnection,
    workflow_run_ids: &[i64],
) -> Result<Vec<workflow_checkpoint::Model>, WorkflowReadError> {
    for workflow_run_id in workflow_run_ids {
        validate_positive_i64(*workflow_run_id, "workflow_run_id")?;
    }
    if workflow_run_ids.is_empty() {
        return Ok(Vec::new());
    }
    workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.is_in(workflow_run_ids.iter().copied()))
        .order_by_asc(workflow_checkpoint::Column::WorkflowRunId)
        .order_by_asc(workflow_checkpoint::Column::StepOrder)
        .order_by_asc(workflow_checkpoint::Column::Id)
        .all(db)
        .await
        .map_err(WorkflowReadError::Database)
}

pub async fn workflow_checkpoint_step_statuses<C>(
    db: &C,
    workflow_run_id: i64,
) -> Result<Vec<(String, String)>, WorkflowReadError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    workflow_checkpoint::Entity::find()
        .select_only()
        .column(workflow_checkpoint::Column::StepKey)
        .column(workflow_checkpoint::Column::Status)
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .order_by_asc(workflow_checkpoint::Column::StepOrder)
        .order_by_asc(workflow_checkpoint::Column::Id)
        .into_tuple::<(String, String)>()
        .all(db)
        .await
        .map_err(WorkflowReadError::Database)
}

pub async fn workflow_checkpoint_statuses_for_runs(
    db: &DatabaseConnection,
    workflow_run_ids: &[i64],
) -> Result<Vec<(i64, String)>, WorkflowReadError> {
    for workflow_run_id in workflow_run_ids {
        validate_positive_i64(*workflow_run_id, "workflow_run_id")?;
    }
    if workflow_run_ids.is_empty() {
        return Ok(Vec::new());
    }
    workflow_checkpoint::Entity::find()
        .select_only()
        .column(workflow_checkpoint::Column::WorkflowRunId)
        .column(workflow_checkpoint::Column::Status)
        .filter(workflow_checkpoint::Column::WorkflowRunId.is_in(workflow_run_ids.iter().copied()))
        .order_by_asc(workflow_checkpoint::Column::WorkflowRunId)
        .order_by_asc(workflow_checkpoint::Column::StepOrder)
        .order_by_asc(workflow_checkpoint::Column::Id)
        .into_tuple::<(i64, String)>()
        .all(db)
        .await
        .map_err(WorkflowReadError::Database)
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
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    let run = locked_workflow_run_by_id(&tx, workflow_run_id).await?;
    ensure_workflow_write_open(&run.status, "删除工作流 checkpoint")?;
    let deleted = workflow_checkpoint::Entity::delete_many()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(workflow_checkpoint::Column::StepKey.is_in(step_keys))
        .exec(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?
        .rows_affected;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(deleted)
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

pub async fn workflow_checkpoint_model_by_step_in<C>(
    db: &C,
    workflow_run_id: i64,
    step_key: &str,
) -> Result<Option<workflow_checkpoint::Model>, WorkflowReadError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    let step_key = required_text(step_key.to_string(), "step_key")?;
    workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(workflow_checkpoint::Column::StepKey.eq(step_key))
        .one(db)
        .await
        .map_err(WorkflowReadError::Database)
}
