use super::*;
use a4n_domain::agent::{AGENT_DEFAULT_MAX_STEPS, AGENT_DEFAULT_MAX_TOOLS, AgentBudget};
use sea_orm::{DatabaseBackend, Statement};
use std::collections::HashMap;

pub struct AgentBudgetSnapshot {
    pub usage: Value,
    pub remaining: AgentBudget,
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
        let usage = own.entry(row.id).or_default();
        for (index, key) in keys.iter().enumerate() {
            if let Some(value) = row.usage_json.get(key) {
                usage[index] = usage[index].max(value.as_u64().ok_or_else(|| {
                    AgentDbError::InvalidArgument(format!("Agent usage.{key} 无效"))
                })?);
            }
        }
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
    let mut ancestor = Some(run_id);
    while let Some(id) = ancestor {
        let row = by_id
            .get(&id)
            .ok_or_else(|| AgentDbError::Conflict("Agent budget lineage 缺失".into()))?;
        let budget: AgentBudget = serde_json::from_value(row.budget_json.clone())
            .map_err(|e| AgentDbError::InvalidArgument(e.to_string()))?;
        let used = totals[&id];
        cap_u32(
            &mut remaining.max_steps,
            budget.max_steps.unwrap_or(AGENT_DEFAULT_MAX_STEPS),
            used[0],
        );
        cap_u32(
            &mut remaining.max_model_calls,
            budget.max_model_calls.unwrap_or(AGENT_DEFAULT_MAX_STEPS),
            used[1],
        );
        cap_u32(
            &mut remaining.max_tool_calls,
            budget.max_tool_calls.unwrap_or(AGENT_DEFAULT_MAX_TOOLS),
            used[2],
        );
        if let Some(limit) = budget.max_tokens {
            cap_u64(
                &mut remaining.max_tokens,
                limit,
                used[3].saturating_add(used[4]),
            );
        }
        if let Some(limit) = budget.max_duration_seconds {
            let elapsed = now()
                .signed_duration_since(row.started_at.unwrap_or(row.created_at))
                .num_seconds()
                .max(0) as u64;
            cap_u64(&mut remaining.max_duration_seconds, limit, elapsed);
        }
        ancestor = row.parent_run_id;
    }
    let mut usage = current.usage_json;
    for (i, key) in keys.iter().enumerate() {
        usage[*key] = json!(totals[&run_id][i]);
    }
    usage["tokens_reserved"] = json!(totals[&run_id][4]);
    Ok(AgentBudgetSnapshot { usage, remaining })
}

pub(super) async fn reserve_budget_on<C: ConnectionTrait>(
    db: &C,
    input: &AgentExecutionInput,
) -> Result<(), AgentDbError> {
    let run = run::Entity::find_by_id(input.run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent Run 不存在".into()))?;
    // Namespace the advisory lock; all descendants share this admission lock.
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT pg_advisory_xact_lock(hashtextextended('a4n-agent-budget:' || $1::text, 0))",
        [run.root_run_id.unwrap_or(run.id).into()],
    ))
    .await?;
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
    let exhausted = (input.execution_kind != "wait" && remaining.max_tokens == Some(0))
        || remaining.max_duration_seconds == Some(0)
        || if input.execution_kind == "model_call" {
            remaining.max_steps == Some(0) || remaining.max_model_calls == Some(0)
        } else {
            remaining.max_tool_calls == Some(0)
        };
    if exhausted {
        return Err(AgentDbError::BudgetExhausted(run.id.to_string()));
    }
    if input.execution_kind == "model_call"
        && let Some(limit) = remaining.max_tokens
    {
        let request = input
            .arguments_json
            .as_ref()
            .ok_or_else(|| AgentDbError::InvalidArgument("模型调用缺少预算请求参数".into()))?;
        if a4n_domain::agent::agent_model_token_reservation(request) > limit {
            return Err(AgentDbError::BudgetExhausted(
                "剩余额度不足以覆盖模型输入和输出".into(),
            ));
        }
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
