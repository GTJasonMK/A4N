use super::*;
use a4n_domain::agent::AgentBudget;
use sea_orm::ConnectionTrait;
use sea_orm::{DatabaseBackend, Statement};
use std::collections::HashMap;

pub struct AgentBudgetSnapshot {
    pub usage: Value,
    pub remaining: AgentBudget,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct BudgetDemand {
    execution_kind: String,
    tokens: u64,
}

impl BudgetDemand {
    pub(super) fn for_execution(input: &AgentExecutionInput) -> Result<Self, AgentDbError> {
        let tokens = if input.execution_kind == "model_call" {
            let request = input
                .arguments_json
                .as_ref()
                .ok_or_else(|| AgentDbError::InvalidArgument("模型调用缺少预算请求参数".into()))?;
            a4n_domain::agent::agent_model_token_reservation(request)
        } else {
            0
        };
        Ok(Self {
            execution_kind: input.execution_kind.clone(),
            tokens,
        })
    }

    fn fits(&self, remaining: &AgentBudget) -> bool {
        remaining.max_duration_seconds != Some(0)
            && (self.execution_kind == "wait" || remaining.max_tokens != Some(0))
            && if self.execution_kind == "model_call" {
                remaining.max_steps != Some(0)
                    && remaining.max_model_calls != Some(0)
                    && remaining
                        .max_tokens
                        .is_none_or(|limit| self.tokens <= limit)
            } else {
                remaining.max_tool_calls != Some(0)
            }
    }
}

pub(super) async fn lock_budget_tree_on<C: ConnectionTrait>(
    db: &C,
    run_id: i64,
) -> Result<AgentRunRecord, AgentDbError> {
    let row = run_by_id(db, run_id)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent Run 不存在".into()))?;
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT pg_advisory_xact_lock(hashtextextended('a4n-agent-budget:' || $1::text, 0))",
        [row.root_run_id.unwrap_or(row.id).into()],
    ))
    .await?;
    Ok(row)
}

pub(super) async fn budget_wait_can_resume<C: ConnectionTrait>(
    db: &C,
    row: &run::Model,
) -> Result<bool, AgentDbError> {
    let remaining = run_budget_snapshot(db, row.id).await?.remaining;
    match row.usage_json.get("budget_wait") {
        Some(value) => Ok(serde_json::from_value::<BudgetDemand>(value.clone())
            .map_err(|error| AgentDbError::InvalidArgument(format!("预算等待参数无效：{error}")))?
            .fits(&remaining)),
        None => Ok(!has_exhausted_limit(&remaining)),
    }
}

/// Derive totals from the existing Run/Execution facts; never copy child usage
/// into a parent counter, where retries could charge it twice or overwrite it.
pub async fn run_budget_snapshot<C: ConnectionTrait>(
    db: &C,
    run_id: i64,
) -> Result<AgentBudgetSnapshot, AgentDbError> {
    let current = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent run 不存在".into()))?;
    let root_id = current.root_run_id.unwrap_or(run_id);
    let rows = run::Entity::find()
        .filter(
            sea_orm::Condition::any()
                .add(run::Column::Id.eq(root_id))
                .add(run::Column::RootRunId.eq(root_id)),
        )
        .all(db)
        .await?;
    let by_id: HashMap<_, _> = rows.iter().map(|r| (r.id, r)).collect();
    let executions = execution::Entity::find()
        .filter(execution::Column::RunId.is_in(by_id.keys().copied()))
        .all(db)
        .await?;
    let mut own: HashMap<i64, [u64; 5]> = HashMap::new();
    for execution in executions {
        let usage = own.entry(execution.run_id).or_default();
        if execution.execution_kind == "model_call" {
            usage[0] += 1;
            usage[1] += 1;
            if let Some(tokens) = execution
                .result_json
                .as_ref()
                .and_then(|v| v.pointer("/usage/total_tokens"))
            {
                usage[3] = usage[3].saturating_add(tokens.as_u64().ok_or_else(|| {
                    AgentDbError::InvalidArgument("模型 usage.total_tokens 无效".into())
                })?);
            } else {
                // Failed/interrupted calls may have consumed tokens before the
                // receipt was lost. Keep their reservation instead of freeing it.
                if let Some(request) = execution.arguments_json.as_ref() {
                    usage[4] = usage[4]
                        .saturating_add(a4n_domain::agent::agent_model_token_reservation(request));
                }
            }
        } else {
            usage[2] += 1;
        }
    }
    let keys = [
        "steps_used",
        "model_calls_used",
        "tool_calls_used",
        "tokens_used",
    ];
    for row in &rows {
        // Only durable reservations/receipts count. Worker progress projections
        // may describe a request still being prepared and are not consumption.
        own.entry(row.id).or_default();
    }
    let mut totals: HashMap<i64, [u64; 5]> = HashMap::new();
    for row in &rows {
        let mut ancestor = Some(row.id);
        let mut seen = HashSet::new();
        while let Some(id) = ancestor {
            if !seen.insert(id) {
                return Err(AgentDbError::Conflict("Agent lineage 存在环".into()));
            }
            let target = by_id
                .get(&id)
                .ok_or_else(|| AgentDbError::Conflict("Agent lineage 缺少祖先".into()))?;
            let total = totals.entry(id).or_default();
            for (i, value) in own[&row.id].iter().enumerate() {
                total[i] = total[i].saturating_add(*value);
            }
            ancestor = target.parent_run_id;
        }
    }
    let mut remaining = AgentBudget::default();
    let mut limiting_run_id = None;
    let mut token_budget_run_id = None;
    let mut deadline_ms: Option<i64> = None;
    let mut ancestor = Some(run_id);
    while let Some(id) = ancestor {
        let row = by_id
            .get(&id)
            .ok_or_else(|| AgentDbError::Conflict("Agent budget lineage 缺失".into()))?;
        let budget = serde_json::from_value::<AgentBudget>(row.budget_json.clone())
            .map_err(|e| AgentDbError::InvalidArgument(e.to_string()))?
            .effective_for_run(row.parent_run_id.is_none());
        let used = totals[&id];
        let mut exhausted_here = false;
        for (target, limit, used) in [
            (&mut remaining.max_steps, budget.max_steps, used[0]),
            (
                &mut remaining.max_model_calls,
                budget.max_model_calls,
                used[1],
            ),
            (
                &mut remaining.max_tool_calls,
                budget.max_tool_calls,
                used[2],
            ),
        ] {
            if let Some(limit) = limit {
                cap_u32(target, limit, used);
                exhausted_here |= used >= u64::from(limit);
            }
        }
        if let Some(limit) = budget.max_tokens {
            let available = limit.saturating_sub(used[3].saturating_add(used[4]));
            if remaining
                .max_tokens
                .is_none_or(|current| available <= current)
            {
                token_budget_run_id = Some(id);
            }
            cap_u64(
                &mut remaining.max_tokens,
                limit,
                used[3].saturating_add(used[4]),
            );
            exhausted_here |= used[3].saturating_add(used[4]) >= limit;
        }
        if let Some(limit) = budget.max_duration_seconds {
            let elapsed = row.started_at.map_or(0, |started| {
                now().signed_duration_since(started).num_seconds().max(0) as u64
            });
            cap_u64(&mut remaining.max_duration_seconds, limit, elapsed);
            exhausted_here |= elapsed >= limit;
            // Closed runs only need totals; active children still inherit ancestor deadlines.
            if let Some(started) = row.started_at
                && current.is_active
            {
                let deadline = i64::try_from(limit)
                    .ok()
                    .and_then(Duration::try_seconds)
                    .and_then(|duration| started.checked_add_signed(duration))
                    .ok_or_else(|| {
                        AgentDbError::InvalidArgument("Agent 时长预算超出时钟范围".into())
                    })?
                    .and_utc()
                    .timestamp_millis();
                deadline_ms = Some(deadline_ms.map_or(deadline, |current| current.min(deadline)));
            }
        }
        if exhausted_here {
            limiting_run_id = Some(id);
        }
        ancestor = row.parent_run_id;
    }
    let mut usage = current.usage_json;
    if !usage.is_object() {
        return Err(AgentDbError::InvalidArgument(
            "Agent usage_json 必须是对象".into(),
        ));
    }
    for (i, key) in keys.iter().enumerate() {
        usage[*key] = json!(totals[&run_id][i]);
    }
    usage["tokens_reserved"] = json!(totals[&run_id][4]);
    usage["usage_complete"] = json!(totals[&run_id][4] == 0);
    if limiting_run_id.is_none() && current.waiting_reason.as_deref() == Some("budget") {
        limiting_run_id = token_budget_run_id;
    }
    usage["budget_limiting_run_id"] = json!(limiting_run_id.map(|id| id.to_string()));
    usage["duration_deadline_unix_ms"] = json!(deadline_ms);
    Ok(AgentBudgetSnapshot { usage, remaining })
}

/// Extends a paused/waiting Run without replacing its execution history.
pub async fn resume_run(
    db: &impl crate::DbExecutor,
    user_id: i64,
    thread_id: i64,
    run_id: i64,
    extension: Option<&AgentBudget>,
) -> Result<AgentRunRecord, AgentDbError> {
    let tx = db.begin().await?;
    let identity = lock_budget_tree_on(&tx, run_id).await?;
    let root = identity.root_run_id.unwrap_or(identity.id);
    let family = run::Entity::find()
        .filter(
            sea_orm::Condition::any()
                .add(run::Column::Id.eq(root))
                .add(run::Column::RootRunId.eq(root)),
        )
        .all(&tx)
        .await?;
    lock_threads_on(&tx, family.iter().map(|run| run.thread_id)).await?;
    lock_runs_on(&tx, family.iter().map(|run| run.id)).await?;
    let owner = thread::Entity::find_by_id(thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .filter(|thread| thread.owner_user_id == user_id)
        .ok_or_else(|| AgentDbError::NotFound("Agent Thread 不存在或无权访问".into()))?;
    let row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .filter(|run| run.thread_id == owner.id)
        .ok_or_else(|| AgentDbError::NotFound("Agent Run 不属于当前 Thread".into()))?;
    let was_paused = matches!(row.status.as_str(), "paused" | "pausing");
    let keep_waiting = (row.status == "waiting"
        && matches!(
            row.waiting_reason.as_deref(),
            Some("approval" | "child" | "external" | "reconcile")
        ))
        || (was_paused
            && matches!(
                row.waiting_reason.as_deref(),
                Some("external" | "reconcile")
            ));
    if !(was_paused
        || keep_waiting && extension.is_some()
        || row.status == "waiting"
            && matches!(
                row.waiting_reason.as_deref(),
                Some("budget" | "user" | "backoff")
            ))
    {
        return Err(AgentDbError::Conflict(
            "当前 Agent 状态不可继续；审批、子任务及对账须先完成".into(),
        ));
    }
    if let Some(extension) = extension {
        let current: AgentBudget = serde_json::from_value(row.budget_json.clone())
            .map_err(|error| AgentDbError::InvalidArgument(error.to_string()))?;
        let next = current
            .extended(extension, row.parent_run_id.is_none())
            .map_err(AgentDbError::InvalidArgument)?;
        let mut active: run::ActiveModel = row.clone().into();
        active.budget_json = Set(serde_json::to_value(next)
            .map_err(|error| AgentDbError::InvalidArgument(error.to_string()))?);
        active.update(&tx).await?;
    }
    let snapshot = run_budget_snapshot(&tx, run_id).await?;
    if !keep_waiting && !budget_wait_can_resume(&tx, &row).await? {
        let limiting = snapshot.usage["budget_limiting_run_id"]
            .as_str()
            .unwrap_or("current");
        return Err(AgentDbError::BudgetExhausted(format!(
            "Run {limiting} 仍无剩余额度，请提高对应预算"
        )));
    }
    let mut resumed = if keep_waiting {
        update_run_state_on(
            &tx,
            run_id,
            None,
            "waiting",
            row.waiting_reason.as_deref(),
            None,
        )
        .await?
    } else {
        update_run_state_on(&tx, run_id, None, "queued", None, None).await?
    };
    refresh_thread_status_on(&tx, thread_id).await?;
    if extension.is_some() || was_paused {
        let parents: HashMap<_, _> = family
            .iter()
            .map(|run| (run.id, run.parent_run_id))
            .collect();
        for child in &family {
            let mut ancestor = child.parent_run_id;
            let mut seen = HashSet::new();
            let mut descendant = false;
            while let Some(id) = ancestor {
                if !seen.insert(id) {
                    return Err(AgentDbError::Conflict("Agent lineage 存在环".into()));
                }
                if id == run_id {
                    descendant = true;
                    break;
                }
                ancestor = parents.get(&id).copied().flatten();
            }
            if !descendant {
                continue;
            }
            let parent_paused = was_paused
                && child.status == "paused"
                && child
                    .usage_json
                    .get("paused_by_run_id")
                    .and_then(Value::as_str)
                    == Some(run_id.to_string().as_str());
            let budget_wait =
                child.status == "waiting" && child.waiting_reason.as_deref() == Some("budget");
            if parent_paused {
                let reason = child
                    .waiting_reason
                    .as_deref()
                    .filter(|reason| matches!(*reason, "external" | "reconcile"));
                let reason = if reason.is_none() && !budget_wait_can_resume(&tx, child).await? {
                    Some("budget")
                } else {
                    reason
                };
                update_run_state_on(
                    &tx,
                    child.id,
                    None,
                    if reason.is_some() {
                        "waiting"
                    } else {
                        "queued"
                    },
                    reason,
                    None,
                )
                .await?;
                refresh_thread_status_on(&tx, child.thread_id).await?;
            } else if budget_wait && budget_wait_can_resume(&tx, child).await? {
                update_run_state_on(&tx, child.id, None, "queued", None, None).await?;
                refresh_thread_status_on(&tx, child.thread_id).await?;
            }
        }
    }
    tx.commit().await?;
    resumed.usage_json = snapshot.usage;
    Ok(resumed.into())
}

fn has_exhausted_limit(remaining: &AgentBudget) -> bool {
    remaining.max_steps == Some(0)
        || remaining.max_model_calls == Some(0)
        || remaining.max_tool_calls == Some(0)
        || remaining.max_tokens == Some(0)
        || remaining.max_duration_seconds == Some(0)
}

pub(super) async fn reserve_budget_on<C: ConnectionTrait>(
    db: &C,
    input: &AgentExecutionInput,
) -> Result<(), AgentDbError> {
    let run = lock_budget_tree_on(db, input.run_id).await?;
    if execution::Entity::find()
        .filter(execution::Column::RunId.eq(run.id))
        .filter(execution::Column::ExecutionKey.eq(&input.execution_key))
        .one(db)
        .await?
        .is_some()
    {
        return Ok(());
    }
    let remaining = run_budget_snapshot(db, run.id).await?.remaining;
    if !BudgetDemand::for_execution(input)?.fits(&remaining) {
        return Err(AgentDbError::BudgetExhausted(
            "剩余额度不足以执行当前请求".into(),
        ));
    }
    Ok(())
}

fn cap_u64(target: &mut Option<u64>, limit: u64, used: u64) {
    let available = limit.saturating_sub(used);
    *target = Some(target.map_or(available, |v| v.min(available)));
}

fn cap_u32(target: &mut Option<u32>, limit: u32, used: u64) {
    let available = u64::from(limit).saturating_sub(used) as u32;
    *target = Some(target.map_or(available, |v| v.min(available)));
}
