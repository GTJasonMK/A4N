use super::super::*;
use sea_orm::sea_query::{Alias, Expr};
use std::collections::{HashMap, HashSet};

pub async fn complete_runs_for_target(
    db: &impl crate::DbExecutor,
    target_table: &str,
    target_id: i64,
    response_summary_json: Option<Value>,
) -> Result<usize, GenerationDbError> {
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
    db: &impl crate::DbExecutor,
    target_table: &str,
    target_id: i64,
    error_message: String,
    response_summary_json: Option<Value>,
) -> Result<usize, GenerationDbError> {
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

pub async fn cancel_runs_for_target(
    db: &impl crate::DbExecutor,
    target_table: &str,
    target_id: i64,
    message: String,
) -> Result<usize, GenerationDbError> {
    update_running_runs_for_target(
        db,
        target_table,
        target_id,
        RUN_STATUS_CANCELLED,
        None,
        Some(message),
    )
    .await
}

/// Batch-cancel running generation runs for multiple target rows in one transaction.
///
/// The returned pairs preserve the caller's target order and report all runs bound to each
/// target, including runs that were already cancelled (the same count as the single-target API).
pub async fn cancel_runs_for_targets(
    db: &impl crate::DbExecutor,
    target_table: &str,
    target_ids: &[i64],
    message: String,
) -> Result<Vec<(i64, usize)>, GenerationDbError> {
    let updates = update_running_runs_for_targets(
        db,
        target_table,
        target_ids,
        RUN_STATUS_CANCELLED,
        None,
        Some(message),
    )
    .await?;
    Ok(target_ids
        .iter()
        .map(|target_id| {
            (
                *target_id,
                updates
                    .target_run_ids
                    .get(target_id)
                    .map_or(0, HashSet::len),
            )
        })
        .collect())
}

pub(crate) async fn interrupt_runs_for_targets(
    db: &impl crate::DbExecutor,
    target_table: &str,
    target_ids: &[i64],
    message: String,
) -> Result<(), GenerationDbError> {
    update_running_runs_for_targets(
        db,
        target_table,
        target_ids,
        RUN_STATUS_INTERRUPTED,
        None,
        Some(message),
    )
    .await
    .map(|_| ())
}

async fn update_running_runs_for_target(
    db: &impl crate::DbExecutor,
    target_table: &str,
    target_id: i64,
    status: &str,
    response_summary_json: Option<Value>,
    error_message: Option<String>,
) -> Result<usize, GenerationDbError> {
    Ok(update_running_runs_for_targets(
        db,
        target_table,
        &[target_id],
        status,
        response_summary_json,
        error_message,
    )
    .await?
    .target_run_ids
    .get(&target_id)
    .map_or(0, HashSet::len))
}

struct TargetRunUpdates {
    target_run_ids: HashMap<i64, HashSet<i64>>,
}

async fn update_running_runs_for_targets(
    db: &impl crate::DbExecutor,
    target_table: &str,
    target_ids: &[i64],
    status: &str,
    response_summary_json: Option<Value>,
    error_message: Option<String>,
) -> Result<TargetRunUpdates, GenerationDbError> {
    let target_table = required_owned_text(target_table.to_string(), "target_table")?;
    for target_id in target_ids {
        validate_positive_id(*target_id, "target_id")?;
    }
    if target_ids.is_empty() {
        return Ok(TargetRunUpdates {
            target_run_ids: HashMap::new(),
        });
    }

    let tx = db.begin().await?;
    let target_rows = generation_run_target::Entity::find()
        .select_only()
        .column(generation_run_target::Column::TargetId)
        .column(generation_run_target::Column::GenerationRunId)
        .filter(generation_run_target::Column::TargetTable.eq(target_table.clone()))
        .filter(generation_run_target::Column::TargetId.is_in(target_ids.iter().copied()))
        .into_tuple::<(i64, i64)>()
        .all(&tx)
        .await?;
    let mut target_run_ids = HashMap::<i64, HashSet<i64>>::new();
    let mut run_ids = HashSet::new();
    for (target_id, run_id) in target_rows {
        target_run_ids.entry(target_id).or_default().insert(run_id);
        run_ids.insert(run_id);
    }
    if run_ids.is_empty() {
        tx.commit().await?;
        return Ok(TargetRunUpdates { target_run_ids });
    }

    let runs = generation_run::Entity::find()
        .select_only()
        .column(generation_run::Column::Id)
        .column(generation_run::Column::Status)
        .filter(generation_run::Column::Id.is_in(run_ids.iter().copied()))
        .order_by_asc(generation_run::Column::CreatedAt)
        .lock_exclusive()
        .into_tuple::<(i64, String)>()
        .all(&tx)
        .await?;
    let mut running_ids = Vec::new();
    for run in &runs {
        if should_update_target_run(&run.1, status)? {
            running_ids.push(run.0);
        }
    }
    let completed_at = Utc::now().naive_utc();
    if !running_ids.is_empty() {
        let updated = generation_run::Entity::update_many()
            .col_expr(
                generation_run::Column::Status,
                Expr::value(status.to_string()),
            )
            .col_expr(
                generation_run::Column::ResponseSummaryJson,
                Expr::value(response_summary_json.clone()).cast_as(Alias::new("json")),
            )
            .col_expr(
                generation_run::Column::ErrorMessage,
                Expr::value(error_message.clone()),
            )
            .col_expr(
                generation_run::Column::CompletedAt,
                Expr::value(Some(completed_at)),
            )
            .filter(generation_run::Column::Id.is_in(running_ids.clone()))
            .exec(&tx)
            .await?;
        if updated.rows_affected != running_ids.len() as u64 {
            return Err(GenerationDbError::InvalidArgument(format!(
                "生成运行终态更新数量不一致: updated={}, expected={}",
                updated.rows_affected,
                running_ids.len()
            )));
        }
    }
    tx.commit().await?;
    Ok(TargetRunUpdates { target_run_ids })
}

fn should_update_target_run(current: &str, target: &str) -> Result<bool, GenerationDbError> {
    if current == RUN_STATUS_RUNNING {
        return Ok(true);
    }
    if current == target {
        return Ok(false);
    }
    Err(GenerationDbError::InvalidArgument(format!(
        "生成运行终态冲突: current={current}, target={target}"
    )))
}

pub(super) async fn update_generation_run_record<C>(
    db: &C,
    run: generation_run::Model,
    status: &str,
    response_summary_json: Option<Value>,
    error_message: Option<String>,
) -> Result<generation_run::Model, GenerationDbError>
where
    C: ConnectionTrait,
{
    let mut active: generation_run::ActiveModel = run.into();
    active.status = Set(status.to_string());
    active.response_summary_json = Set(response_summary_json);
    active.error_message = Set(error_message);
    active.completed_at = Set(Some(Utc::now().naive_utc()));
    active.update(db).await.map_err(GenerationDbError::Database)
}

#[cfg(test)]
mod tests {
    use super::should_update_target_run;

    #[test]
    fn target_run_transition_is_idempotent_but_rejects_other_terminal_status() {
        assert!(should_update_target_run("running", "cancelled").unwrap());
        assert!(!should_update_target_run("cancelled", "cancelled").unwrap());
        assert!(should_update_target_run("completed", "cancelled").is_err());
    }
}
