use super::super::*;
use std::collections::{HashMap, HashSet};

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
    let run_ids = generation_run_target::Entity::find()
        .select_only()
        .column(generation_run_target::Column::GenerationRunId)
        .filter(generation_run_target::Column::TargetTable.eq(target_table))
        .filter(generation_run_target::Column::TargetId.eq(target_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    if run_ids.is_empty() {
        return Ok(false);
    }
    let run_ids = run_ids.into_iter().collect::<HashSet<_>>();
    let summaries = generation_run::Entity::find()
        .select_only()
        .column(generation_run::Column::ResponseSummaryJson)
        .filter(generation_run::Column::Id.is_in(run_ids))
        .filter(generation_run::Column::StepKey.eq(step_key))
        .into_tuple::<Option<Value>>()
        .all(db)
        .await?;
    Ok(summaries.into_iter().any(|summary| {
        summary
            .as_ref()
            .and_then(|summary| summary.get("output"))
            .and_then(Value::as_str)
            == Some(response_output)
    }))
}

pub(crate) async fn generation_run_existing_target_outputs<C>(
    db: &C,
    target_table: &str,
    target_ids: &[i64],
    step_keys: &[String],
) -> Result<HashSet<(i64, String, String)>, GenerationDbError>
where
    C: ConnectionTrait,
{
    if target_ids.is_empty() || step_keys.is_empty() {
        return Ok(HashSet::new());
    }
    let target_table = required_text(target_table, "target_table")?.to_string();
    for target_id in target_ids {
        validate_positive_id(*target_id, "target_id")?;
    }
    let step_keys = step_keys
        .iter()
        .map(|step_key| required_text(step_key, "step_key").map(str::to_string))
        .collect::<Result<Vec<_>, _>>()?;

    let target_rows = generation_run_target::Entity::find()
        .select_only()
        .column(generation_run_target::Column::TargetId)
        .column(generation_run_target::Column::GenerationRunId)
        .filter(generation_run_target::Column::TargetTable.eq(target_table))
        .filter(generation_run_target::Column::TargetId.is_in(target_ids.iter().copied()))
        .into_tuple::<(i64, i64)>()
        .all(db)
        .await?;
    if target_rows.is_empty() {
        return Ok(HashSet::new());
    }

    let run_ids = target_rows
        .iter()
        .map(|(_, run_id)| *run_id)
        .collect::<HashSet<_>>();
    let runs = generation_run::Entity::find()
        .select_only()
        .column(generation_run::Column::Id)
        .column(generation_run::Column::StepKey)
        .column(generation_run::Column::ResponseSummaryJson)
        .filter(generation_run::Column::Id.is_in(run_ids))
        .filter(generation_run::Column::StepKey.is_in(step_keys))
        .into_tuple::<(i64, String, Option<Value>)>()
        .all(db)
        .await?;
    let mut run_outputs = HashMap::with_capacity(runs.len());
    for (run_id, step_key, summary) in runs {
        if let Some(output) = summary
            .as_ref()
            .and_then(|summary| summary.get("output"))
            .and_then(Value::as_str)
        {
            run_outputs.insert(run_id, (step_key, output.to_string()));
        }
    }
    Ok(target_rows
        .into_iter()
        .filter_map(|(target_id, run_id)| {
            run_outputs
                .get(&run_id)
                .map(|(step_key, output)| (target_id, step_key.clone(), output.clone()))
        })
        .collect())
}

pub(super) async fn insert_generation_run_targets<C>(
    db: &C,
    generation_run_id: i64,
    targets: Vec<GenerationRunTraceTarget>,
) -> Result<(), GenerationDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(generation_run_id, "generation_run_id")?;
    if targets.is_empty() {
        return Err(GenerationDbError::InvalidArgument(
            "生成运行记录必须绑定至少一个目标资产".to_string(),
        ));
    }
    let target_count = targets.len();
    let normalized_targets = targets
        .into_iter()
        .map(|target| {
            validate_positive_id(target.target_id, "target_id")?;
            Ok::<_, GenerationDbError>(generation_run_target::ActiveModel {
                generation_run_id: Set(generation_run_id),
                target_kind: Set(required_owned_text(target.target_kind, "target_kind")?),
                target_table: Set(required_owned_text(target.target_table, "target_table")?),
                target_id: Set(target.target_id),
                role: Set("output".to_string()),
                created_at: Set(Utc::now().naive_utc()),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, GenerationDbError>>()?;
    generation_run::Entity::find_by_id(generation_run_id)
        .select_only()
        .column(generation_run::Column::Id)
        .into_tuple::<i64>()
        .one(db)
        .await?
        .ok_or_else(|| {
            GenerationDbError::NotFound(format!("生成运行记录不存在: {generation_run_id}"))
        })?;
    let inserted = generation_run_target::Entity::insert_many(normalized_targets)
        .exec_without_returning(db)
        .await?;
    if inserted != target_count as u64 {
        return Err(GenerationDbError::InvalidArgument(format!(
            "生成运行目标写入数量不一致: inserted={inserted}, expected={target_count}"
        )));
    }
    Ok(())
}
