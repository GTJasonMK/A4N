use super::super::*;

pub async fn complete_runs_for_target(
    db: &DatabaseConnection,
    target_table: &str,
    target_id: i64,
    response_summary_json: Option<Value>,
) -> Result<Vec<generation_run::Model>, GenerationDbError> {
    update_running_runs_for_target(
        db,
        target_table,
        target_id,
        RUN_STATUS_COMPLETED,
        response_summary_json,
        None,
    )
    .await
}

pub async fn fail_runs_for_target(
    db: &DatabaseConnection,
    target_table: &str,
    target_id: i64,
    error_message: String,
    response_summary_json: Option<Value>,
) -> Result<Vec<generation_run::Model>, GenerationDbError> {
    update_running_runs_for_target(
        db,
        target_table,
        target_id,
        RUN_STATUS_FAILED,
        response_summary_json,
        Some(error_message),
    )
    .await
}

async fn update_running_runs_for_target(
    db: &DatabaseConnection,
    target_table: &str,
    target_id: i64,
    status: &str,
    response_summary_json: Option<Value>,
    error_message: Option<String>,
) -> Result<Vec<generation_run::Model>, GenerationDbError> {
    let target_table = required_owned_text(target_table.to_string(), "target_table")?;
    validate_positive_id(target_id, "target_id")?;
    let tx = db.begin().await?;
    let targets = generation_run_target::Entity::find()
        .filter(generation_run_target::Column::TargetTable.eq(target_table))
        .filter(generation_run_target::Column::TargetId.eq(target_id))
        .order_by_asc(generation_run_target::Column::Id)
        .all(&tx)
        .await?;
    if targets.is_empty() {
        return Ok(Vec::new());
    }
    let run_ids = targets
        .iter()
        .map(|target| target.generation_run_id)
        .collect::<BTreeSet<_>>();
    let runs = generation_run::Entity::find()
        .filter(generation_run::Column::Id.is_in(run_ids))
        .filter(generation_run::Column::Status.eq(RUN_STATUS_RUNNING))
        .order_by_asc(generation_run::Column::CreatedAt)
        .all(&tx)
        .await?;
    let mut updated = Vec::with_capacity(runs.len());
    for run in runs {
        updated.push(
            update_generation_run_status(
                &tx,
                run.id,
                status,
                response_summary_json.clone(),
                error_message.clone(),
            )
            .await?,
        );
    }
    tx.commit().await?;
    Ok(updated)
}

pub(super) async fn update_generation_run_status<C>(
    db: &C,
    generation_run_id: i64,
    status: &str,
    response_summary_json: Option<Value>,
    error_message: Option<String>,
) -> Result<generation_run::Model, GenerationDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(generation_run_id, "generation_run_id")?;
    let run = generation_run::Entity::find_by_id(generation_run_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            GenerationDbError::NotFound(format!("生成运行记录不存在: {generation_run_id}"))
        })?;
    let mut active: generation_run::ActiveModel = run.into();
    active.status = Set(status.to_string());
    active.response_summary_json = Set(response_summary_json);
    active.error_message = Set(error_message);
    active.completed_at = Set(Some(Utc::now().naive_utc()));
    active.update(db).await.map_err(GenerationDbError::Database)
}
