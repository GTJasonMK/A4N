use super::*;
use a4n_db_models::models::agent::inbox as inbox_model;

pub async fn recover_expired_run(
    db: &DatabaseConnection,
    run_id: i64,
    error_message: Option<&str>,
) -> Result<run::Model, AgentDbError> {
    let current = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let parent_identity = if let Some(parent_run_id) = current.parent_run_id {
        let delegation = delegation::Entity::find()
            .filter(delegation::Column::ChildRunId.eq(current.id))
            .one(db)
            .await?
            .ok_or_else(|| {
                AgentDbError::Conflict(format!(
                    "Agent child run 缺少 delegation，无法恢复: {}",
                    current.id
                ))
            })?;
        if delegation.parent_run_id != parent_run_id {
            return Err(AgentDbError::Conflict(format!(
                "Agent child delegation parent 不一致: {}",
                current.id
            )));
        }
        Some(
            run::Entity::find_by_id(parent_run_id)
                .one(db)
                .await?
                .ok_or_else(|| {
                    AgentDbError::NotFound(format!("Agent parent run 不存在: {parent_run_id}"))
                })?,
        )
    } else {
        None
    };
    let tx = db.begin().await?;
    let mut thread_ids = vec![current.thread_id];
    if let Some(parent) = parent_identity.as_ref() {
        thread_ids.push(parent.thread_id);
    }
    lock_threads_on(&tx, thread_ids.iter().copied()).await?;
    let run_ids = parent_identity
        .as_ref()
        .map(|parent| vec![current.id, parent.id])
        .unwrap_or_else(|| vec![current.id]);
    let locked_runs = lock_runs_on(&tx, run_ids).await?;
    let row = locked_runs
        .iter()
        .find(|row| row.id == run_id)
        .cloned()
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    if row.status == "recovery_required" && !row.is_active {
        let _ = refresh_thread_status_on(&tx, row.thread_id).await?;
        tx.commit().await?;
        return Ok(row);
    }
    let timestamp = now();
    if !row.is_active
        || row.status != "running"
        || row
            .lease_expires_at
            .is_none_or(|expires_at| expires_at > timestamp)
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent run 不满足过期恢复条件: {run_id}"
        )));
    }
    let parent_run = parent_identity
        .and_then(|parent| locked_runs.iter().find(|row| row.id == parent.id).cloned());
    let mut active: run::ActiveModel = row.into();
    active.status = Set("recovery_required".to_string());
    active.waiting_reason = Set(Some("reconcile".to_string()));
    active.is_active = Set(false);
    active.last_error = Set(error_message.map(str::to_string));
    active.worker_id = Set(None);
    active.lease_token = Set(None);
    active.lease_expires_at = Set(None);
    active.updated_at = Set(timestamp);
    active.completed_at = Set(Some(timestamp));
    let row = active.update(&tx).await?;
    terminate_executions_on(
        &tx,
        [row.id],
        ExecutionTermination::RecoveryRequired,
        row.last_error.as_deref(),
        timestamp,
    )
    .await?;
    interrupt_streaming_messages_on(&tx, [row.id], row.last_error.as_deref()).await?;
    append_run_event_on(
        &tx,
        &row,
        "run_recovery_required",
        "recovery_required",
        "Agent run 需要显式恢复或对账",
        None,
        None,
        None,
        row.last_error
            .clone()
            .map(|error| json!({ "error": error })),
    )
    .await?;
    if let Some(parent_run) = parent_run {
        recover_child_delegation_on(&tx, &row, &parent_run, error_message, timestamp).await?;
    }
    for thread_id in thread_ids {
        let _ = refresh_thread_status_on(&tx, thread_id).await?;
    }
    tx.commit().await?;
    Ok(row)
}

/// Lists expired running leases for the recovery worker.
///
/// This is deliberately a read-only scan: `recover_expired_run` performs the
/// exclusive lock and revalidates expiry before changing durable state.
pub async fn expired_running_runs<C>(db: &C, limit: u64) -> Result<Vec<run::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    if limit == 0 {
        return Err(AgentDbError::InvalidArgument(
            "expired running run limit 必须大于 0".into(),
        ));
    }
    run::Entity::find()
        .filter(run::Column::IsActive.eq(true))
        .filter(run::Column::Status.eq("running"))
        .filter(run::Column::LeaseExpiresAt.lte(now()))
        .order_by_asc(run::Column::LeaseExpiresAt)
        .order_by_asc(run::Column::Id)
        .limit(limit)
        .all(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn cancel_run_tree(
    db: &DatabaseConnection,
    root_run_id: i64,
    error_message: Option<&str>,
) -> Result<Vec<run::Model>, AgentDbError> {
    let root = run::Entity::find_by_id(root_run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {root_run_id}")))?;
    let tree_root_id = root.root_run_id.unwrap_or(root.id);
    let tree_rows = run::Entity::find()
        .filter(
            sea_orm::Condition::any()
                .add(run::Column::Id.eq(tree_root_id))
                .add(run::Column::RootRunId.eq(tree_root_id)),
        )
        .all(db)
        .await?;
    let parent_by_run = tree_rows
        .iter()
        .map(|row| (row.id, row.parent_run_id))
        .collect::<std::collections::HashMap<_, _>>();
    let visited = tree_rows
        .iter()
        .filter(|row| {
            let mut current = Some(row.id);
            let mut seen = HashSet::new();
            while let Some(run_id) = current {
                if run_id == root_run_id {
                    return true;
                }
                if !seen.insert(run_id) {
                    return false;
                }
                current = parent_by_run.get(&run_id).copied().flatten();
            }
            false
        })
        .map(|row| row.id)
        .collect::<HashSet<_>>();
    if !visited.contains(&root_run_id) {
        return Err(AgentDbError::Conflict(format!(
            "Agent run lineage 已损坏: {root_run_id}"
        )));
    }
    let thread_ids = tree_rows
        .iter()
        .map(|row| row.thread_id)
        .collect::<HashSet<_>>();
    let delegation_rows = delegation::Entity::find()
        .filter(
            sea_orm::Condition::any()
                .add(delegation::Column::ParentRunId.is_in(visited.iter().copied()))
                .add(delegation::Column::ChildRunId.is_in(visited.iter().copied())),
        )
        .all(db)
        .await?;
    let delegation_ids = delegation_rows.iter().map(|row| row.id).collect::<Vec<_>>();
    // When a child is cancelled directly, its parent is outside `visited`.
    // Lock those parent Threads/Runs in the same transaction so cancellation
    // can atomically deliver a tool observation and wake the waiting parent.
    let boundary_parent_run_ids = delegation_rows
        .iter()
        .filter_map(|row| {
            row.child_run_id
                .filter(|child_id| visited.contains(child_id))
                .and_then(|_| (!visited.contains(&row.parent_run_id)).then_some(row.parent_run_id))
        })
        .collect::<HashSet<_>>();
    let boundary_parent_rows = if boundary_parent_run_ids.is_empty() {
        Vec::new()
    } else {
        run::Entity::find()
            .filter(run::Column::Id.is_in(boundary_parent_run_ids.iter().copied()))
            .all(db)
            .await?
    };
    if boundary_parent_rows.len() != boundary_parent_run_ids.len() {
        return Err(AgentDbError::Conflict(
            "Agent child cancellation 缺少 parent run".into(),
        ));
    }
    let mut lock_thread_ids = thread_ids.clone();
    lock_thread_ids.extend(boundary_parent_rows.iter().map(|row| row.thread_id));

    let tx = db.begin().await?;
    lock_threads_on(&tx, lock_thread_ids.iter().copied()).await?;
    // Re-scan after every candidate Thread is locked. Child creation follows
    // the same Thread-before-Run order, so no new descendant can cross this
    // point. If the pre-scan missed one, abort the transaction and retry with
    // the expanded lock set instead of leaving a live child behind.
    let locked_tree_rows = run::Entity::find()
        .filter(
            sea_orm::Condition::any()
                .add(run::Column::Id.eq(tree_root_id))
                .add(run::Column::RootRunId.eq(tree_root_id)),
        )
        .all(&tx)
        .await?;
    let locked_parent_by_run = locked_tree_rows
        .iter()
        .map(|row| (row.id, row.parent_run_id))
        .collect::<std::collections::HashMap<_, _>>();
    let locked_visited = locked_tree_rows
        .iter()
        .filter(|row| {
            let mut current = Some(row.id);
            let mut seen = HashSet::new();
            while let Some(run_id) = current {
                if run_id == root_run_id {
                    return true;
                }
                if !seen.insert(run_id) {
                    return false;
                }
                current = locked_parent_by_run.get(&run_id).copied().flatten();
            }
            false
        })
        .map(|row| row.id)
        .collect::<HashSet<_>>();
    let locked_thread_ids = locked_tree_rows
        .iter()
        .map(|row| row.thread_id)
        .collect::<HashSet<_>>();
    if locked_visited != visited || !locked_thread_ids.is_subset(&lock_thread_ids) {
        tx.rollback().await?;
        return Box::pin(cancel_run_tree(db, root_run_id, error_message)).await;
    }
    let mut lock_run_ids = visited.clone();
    lock_run_ids.extend(boundary_parent_run_ids.iter().copied());
    let locked_runs = lock_runs_on(&tx, lock_run_ids.iter().copied()).await?;
    let mut rows = locked_runs
        .iter()
        .filter(|row| visited.contains(&row.id))
        .cloned()
        .collect::<Vec<_>>();
    let mut boundary_parent_runs = locked_runs
        .iter()
        .filter(|row| boundary_parent_run_ids.contains(&row.id))
        .map(|row| (row.id, row.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    let _locked_turns = lock_turns_on(&tx, locked_runs.iter().map(|row| row.turn_id)).await?;
    let locked_delegation_ids = delegation::Entity::find()
        .select_only()
        .column(delegation::Column::Id)
        .filter(
            sea_orm::Condition::any()
                .add(delegation::Column::ParentRunId.is_in(visited.iter().copied()))
                .add(delegation::Column::ChildRunId.is_in(visited.iter().copied())),
        )
        .into_tuple::<i64>()
        .all(&tx)
        .await?;
    if locked_delegation_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>()
        != delegation_ids.iter().copied().collect::<HashSet<_>>()
    {
        tx.rollback().await?;
        return Box::pin(cancel_run_tree(db, root_run_id, error_message)).await;
    }
    let locked_delegations = lock_delegations_on(&tx, locked_delegation_ids).await?;
    if !rows.iter().any(|row| row.id == root.id) {
        return Err(AgentDbError::NotFound(format!(
            "Agent run 不存在: {root_run_id}"
        )));
    }
    let timestamp = now();
    let mut cancelled = Vec::with_capacity(rows.len());
    let mut turn_ids = HashSet::new();
    let mut tree_run_ids = Vec::with_capacity(rows.len());
    for row in rows.drain(..) {
        turn_ids.insert(row.turn_id);
        tree_run_ids.push(row.id);
        if is_run_terminal(&row.status) || !row.is_active {
            cancelled.push(row);
            continue;
        }
        let mut active: run::ActiveModel = row.into();
        active.status = Set("cancelled".to_string());
        active.waiting_reason = Set(None);
        active.is_active = Set(false);
        active.last_error = Set(error_message.map(str::to_string));
        active.worker_id = Set(None);
        active.lease_token = Set(None);
        active.lease_expires_at = Set(None);
        active.updated_at = Set(timestamp);
        active.completed_at = Set(Some(timestamp));
        let updated = active.update(&tx).await?;
        append_run_event_on(
            &tx,
            &updated,
            "run_cancelled",
            "cancelled",
            "Agent run 已取消",
            None,
            None,
            None,
            error_message.map(|error| json!({ "error": error })),
        )
        .await?;
        cancelled.push(updated);
    }

    if !turn_ids.is_empty() {
        turn::Entity::update_many()
            .col_expr(
                turn::Column::Status,
                sea_orm::sea_query::Expr::value("cancelled"),
            )
            .col_expr(
                turn::Column::ErrorMessage,
                sea_orm::sea_query::Expr::value(error_message.map(str::to_string)),
            )
            .col_expr(
                turn::Column::UpdatedAt,
                sea_orm::sea_query::Expr::value(timestamp),
            )
            .col_expr(
                turn::Column::CompletedAt,
                sea_orm::sea_query::Expr::value(Some(timestamp)),
            )
            .filter(turn::Column::Id.is_in(turn_ids.iter().copied()))
            .filter(turn::Column::CompletedAt.is_null())
            .exec(&tx)
            .await?;
    }
    if !tree_run_ids.is_empty() {
        terminate_executions_on(
            &tx,
            tree_run_ids.iter().copied(),
            ExecutionTermination::Cancelled,
            error_message,
            timestamp,
        )
        .await?;
        interrupt_streaming_messages_on(&tx, tree_run_ids.iter().copied(), error_message).await?;
        for row in locked_delegations {
            if is_delegation_terminal(&row.status) {
                continue;
            }
            let boundary_child_run_id = row.child_run_id.filter(|child_run_id| {
                visited.contains(child_run_id) && !visited.contains(&row.parent_run_id)
            });
            let boundary_child_run = boundary_child_run_id
                .and_then(|child_run_id| cancelled.iter().find(|run| run.id == child_run_id));
            // Cancelling an already-terminal child is an idempotent no-op. In
            // particular, do not turn a completed child awaiting parent
            // acceptance into a cancelled delegation.
            if boundary_child_run_id.is_some()
                && boundary_child_run.is_none_or(|run| run.status != "cancelled")
            {
                continue;
            }
            let mut active: delegation::ActiveModel = row.into();
            active.status = Set("cancelled".to_string());
            active.error_message = Set(error_message.map(str::to_string));
            active.updated_at = Set(timestamp);
            active.completed_at = Set(Some(timestamp));
            let delegation = active.update(&tx).await?;

            let Some(child_run) = boundary_child_run else {
                continue;
            };
            let parent_run = boundary_parent_runs
                .get(&delegation.parent_run_id)
                .cloned()
                .ok_or_else(|| {
                    AgentDbError::Conflict(format!(
                        "Agent child cancellation 缺少 parent run: {}",
                        delegation.parent_run_id
                    ))
                })?;
            if !parent_run.is_active || is_run_terminal(&parent_run.status) {
                return Err(AgentDbError::Conflict(format!(
                    "Agent child cancellation 无法投递给已终止 parent run: {}",
                    parent_run.id
                )));
            }
            if child_run.thread_id != delegation.child_thread_id
                || parent_run.thread_id != delegation.parent_thread_id
            {
                return Err(AgentDbError::Conflict(format!(
                    "Agent child cancellation delegation lineage 不一致: {}",
                    delegation.id
                )));
            }

            let reason = error_message
                .unwrap_or("Agent child run 已取消")
                .to_string();
            let parent_tool_message = if let Some(execution_id) = delegation.parent_execution_id {
                let parent_execution = execution::Entity::find_by_id(execution_id)
                    .lock_exclusive()
                    .one(&tx)
                    .await?
                    .ok_or_else(|| {
                        AgentDbError::NotFound(format!(
                            "Agent parent delegation execution 不存在: {execution_id}"
                        ))
                    })?;
                let result = json!({
                    "delegation_id": delegation.id.to_string(),
                    "child_run_id": child_run.id.to_string(),
                    "cancelled": true,
                    "reason": reason.clone(),
                });
                let message = append_parent_delegation_tool_message_on(
                    &tx,
                    &parent_run,
                    child_run,
                    &delegation,
                    &parent_execution,
                    "cancelled",
                    result.clone(),
                )
                .await?;
                finish_parent_delegation_execution_on(
                    &tx,
                    parent_execution,
                    "cancelled",
                    result,
                    "Child Agent 已取消".into(),
                    None,
                    Some(reason.clone()),
                    timestamp,
                )
                .await?;
                Some(message)
            } else {
                None
            };

            let mut inbox_payload = json!({
                "delegation_id": delegation.id.to_string(),
                "child_run_id": child_run.id.to_string(),
                "cancelled": true,
                "reason": reason.clone(),
            });
            if let Some(message) = parent_tool_message.as_ref() {
                inbox_payload["message_id"] = json!(message.id.to_string());
            }
            enqueue_inbox(
                &tx,
                AgentInboxInput {
                    thread_id: parent_run.thread_id,
                    turn_id: Some(parent_run.turn_id),
                    run_id: Some(parent_run.id),
                    source_kind: "subagent".into(),
                    source_thread_id: Some(child_run.thread_id),
                    inbox_kind: "child_result".into(),
                    delivery_mode: "followup".into(),
                    priority: 20,
                    dedupe_key: format!("child-cancelled:{}", delegation.id),
                    content: format!("Child Agent cancelled: {reason}"),
                    payload_json: Some(inbox_payload),
                    available_at: None,
                },
            )
            .await?;

            if parent_run.status == "waiting"
                && parent_run.waiting_reason.as_deref() == Some("child")
            {
                let mut parent_active: run::ActiveModel = parent_run.into();
                parent_active.status = Set("queued".to_string());
                parent_active.waiting_reason = Set(None);
                parent_active.worker_id = Set(None);
                parent_active.lease_token = Set(None);
                parent_active.lease_expires_at = Set(None);
                parent_active.next_attempt_at = Set(timestamp);
                parent_active.updated_at = Set(timestamp);
                let parent_run = parent_active.update(&tx).await?;
                boundary_parent_runs.insert(parent_run.id, parent_run);
            }
        }
        approval::Entity::update_many()
            .col_expr(
                approval::Column::Status,
                sea_orm::sea_query::Expr::value("rejected"),
            )
            .col_expr(
                approval::Column::DecisionNote,
                sea_orm::sea_query::Expr::value(Some("Agent run 已取消".to_string())),
            )
            .col_expr(
                approval::Column::DecidedAt,
                sea_orm::sea_query::Expr::value(Some(timestamp)),
            )
            .filter(approval::Column::RunId.is_in(tree_run_ids.iter().copied()))
            .filter(approval::Column::Status.eq("pending"))
            .exec(&tx)
            .await?;
        inbox_model::Entity::update_many()
            .col_expr(
                inbox_model::Column::Status,
                sea_orm::sea_query::Expr::value("cancelled"),
            )
            .col_expr(
                inbox_model::Column::ClaimToken,
                sea_orm::sea_query::Expr::value(Option::<String>::None),
            )
            .col_expr(
                inbox_model::Column::ClaimExpiresAt,
                sea_orm::sea_query::Expr::value(Option::<NaiveDateTime>::None),
            )
            .col_expr(
                inbox_model::Column::ConsumedAt,
                sea_orm::sea_query::Expr::value(Some(timestamp)),
            )
            .filter(inbox_model::Column::RunId.is_in(tree_run_ids.iter().copied()))
            .filter(inbox_model::Column::Status.is_in(["pending", "claimed"]))
            .exec(&tx)
            .await?;
    }
    for thread_id in lock_thread_ids {
        let _ = refresh_thread_status_on(&tx, thread_id).await?;
    }
    tx.commit().await?;
    Ok(cancelled)
}
