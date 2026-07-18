use super::super::*;

pub async fn delete_generation_run_targets_for<C>(
    db: &C,
    target_table: &str,
    target_ids: &[i64],
) -> Result<u64, GenerationDbError>
where
    C: ConnectionTrait,
{
    if target_ids.is_empty() {
        return Ok(0);
    }
    let target_table = required_text(target_table, "target_table")?.to_string();
    for target_id in target_ids {
        validate_positive_id(*target_id, "target_id")?;
    }

    let result = generation_run_target::Entity::delete_many()
        .filter(generation_run_target::Column::TargetTable.eq(target_table))
        .filter(generation_run_target::Column::TargetId.is_in(target_ids.iter().copied()))
        .exec(db)
        .await?;

    Ok(result.rows_affected)
}

pub async fn generation_run_exists_for_target_output<C>(
    db: &C,
    target_table: &str,
    target_id: i64,
    step_key: &str,
    response_output: &str,
) -> Result<bool, GenerationDbError>
where
    C: ConnectionTrait,
{
    let target_table = required_text(target_table, "target_table")?.to_string();
    validate_positive_id(target_id, "target_id")?;
    let step_key = required_text(step_key, "step_key")?.to_string();
    let targets = generation_run_target::Entity::find()
        .filter(generation_run_target::Column::TargetTable.eq(target_table))
        .filter(generation_run_target::Column::TargetId.eq(target_id))
        .all(db)
        .await?;
    if targets.is_empty() {
        return Ok(false);
    }
    let run_ids = targets
        .iter()
        .map(|target| target.generation_run_id)
        .collect::<BTreeSet<_>>();
    let runs = generation_run::Entity::find()
        .filter(generation_run::Column::Id.is_in(run_ids))
        .filter(generation_run::Column::StepKey.eq(step_key))
        .all(db)
        .await?;
    Ok(runs.into_iter().any(|run| {
        run.response_summary_json
            .as_ref()
            .and_then(|summary| summary.get("output"))
            .and_then(Value::as_str)
            == Some(response_output)
    }))
}

pub(super) async fn insert_generation_run_target<C>(
    db: &C,
    input: AttachGenerationRunTargetInput,
) -> Result<generation_run_target::Model, GenerationDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(input.generation_run_id, "generation_run_id")?;
    validate_positive_id(input.target_id, "target_id")?;
    let target_kind = required_owned_text(input.target_kind, "target_kind")?;
    let target_table = required_owned_text(input.target_table, "target_table")?;
    let role = required_owned_text(input.role, "role")?;
    let run = generation_run::Entity::find_by_id(input.generation_run_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            GenerationDbError::NotFound(format!("生成运行记录不存在: {}", input.generation_run_id))
        })?;

    generation_run_target::ActiveModel {
        generation_run_id: Set(run.id),
        target_kind: Set(target_kind),
        target_table: Set(target_table),
        target_id: Set(input.target_id),
        role: Set(role),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(GenerationDbError::Database)
}
