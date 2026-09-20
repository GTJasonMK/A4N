//! Agent budget snapshots and parent/child budget enforcement.

use super::*;

#[derive(Debug, Clone)]
pub struct AgentWorkerBudgetSnapshot {
    pub usage: Value,
    pub max_tokens: Option<u64>,
    pub max_duration_seconds: Option<u64>,
}

impl From<AgentWorkerBudgetSnapshot> for AgentWorkerBudgetSnapshotWire {
    fn from(snapshot: AgentWorkerBudgetSnapshot) -> Self {
        Self {
            usage: snapshot.usage,
            max_tokens: snapshot.max_tokens,
            max_duration_seconds: snapshot.max_duration_seconds,
        }
    }
}

impl From<AgentWorkerBudgetSnapshotWire> for AgentWorkerBudgetSnapshot {
    fn from(snapshot: AgentWorkerBudgetSnapshotWire) -> Self {
        Self {
            usage: snapshot.usage,
            max_tokens: snapshot.max_tokens,
            max_duration_seconds: snapshot.max_duration_seconds,
        }
    }
}

pub fn validate_agent_budget(budget: &AgentBudget) -> Result<(), AppError> {
    let positive = [
        budget.max_duration_seconds,
        budget.max_tokens,
        budget.max_cost_micros,
    ]
    .into_iter()
    .flatten()
    .all(|value| value > 0)
        && [
            budget.max_steps,
            budget.max_model_calls,
            budget.max_tool_calls,
        ]
        .into_iter()
        .flatten()
        .all(|value| value > 0)
        && [budget.max_parallel_tools, budget.max_children]
            .into_iter()
            .flatten()
            .all(|value| value > 0)
        && budget.max_delegation_depth.is_none_or(|value| value > 0);
    if !positive {
        return Err(AppError::InvalidArgument(
            "Agent budget 上限必须为正数".into(),
        ));
    }
    if budget
        .max_duration_seconds
        .is_some_and(|value| value > i64::MAX as u64)
    {
        return Err(AppError::InvalidArgument(
            "Agent max_duration_seconds 超出时间实现上限".into(),
        ));
    }
    if budget.max_cost_micros.is_some() {
        return Err(AppError::InvalidArgument(
            "当前 provider 没有可靠的 Agent cost usage，暂不支持 max_cost_micros".into(),
        ));
    }
    Ok(())
}

pub fn enforce_delegation_limits(
    delegations: &[AgentDelegationSnapshot],
    run_id: i64,
    depth: u8,
    budget: &AgentBudget,
) -> Result<(), AppError> {
    if budget.max_children.is_some_and(|limit| {
        delegations
            .iter()
            .filter(|row| row.parent_run_id == run_id)
            .count()
            >= usize::from(limit)
    }) {
        return Err(AppError::InvalidState {
            current: format!("Agent run {run_id} 已达到 max_children"),
            operation: "创建 child Agent".into(),
        });
    }
    if budget
        .max_delegation_depth
        .is_some_and(|limit| depth >= limit)
    {
        return Err(AppError::InvalidState {
            current: format!("Agent run {run_id} 已达到 max_delegation_depth"),
            operation: "创建 child Agent".into(),
        });
    }
    Ok(())
}

pub fn child_budget_within_parent(
    mut child: AgentBudget,
    parent: &AgentBudget,
    usage: &Value,
) -> Result<AgentBudget, AppError> {
    validate_agent_budget(&child)?;
    let steps_used = budget_usage_u32(usage, "steps_used")?;
    let model_calls_used = budget_usage_u32(usage, "model_calls_used")?;
    let tool_calls_used = budget_usage_u32(usage, "tool_calls_used")?;
    let tokens_used = budget_usage_u64(usage, "tokens_used")?;
    ensure_budget_remaining_u32(parent.max_steps, steps_used, "max_steps")?;
    ensure_budget_remaining_u32(parent.max_model_calls, model_calls_used, "max_model_calls")?;
    ensure_budget_remaining_u32(parent.max_tool_calls, tool_calls_used, "max_tool_calls")?;
    ensure_budget_remaining_u64(parent.max_tokens, tokens_used, "max_tokens")?;
    cap_budget_u32(&mut child.max_steps, parent.max_steps, steps_used);
    cap_budget_u32(
        &mut child.max_model_calls,
        parent.max_model_calls,
        model_calls_used,
    );
    cap_budget_u32(
        &mut child.max_tool_calls,
        parent.max_tool_calls,
        tool_calls_used,
    );
    cap_budget_u64(&mut child.max_tokens, parent.max_tokens, tokens_used);
    cap_budget_u64(
        &mut child.max_duration_seconds,
        parent.max_duration_seconds,
        0,
    );
    cap_budget_u16(&mut child.max_parallel_tools, parent.max_parallel_tools);
    cap_budget_u16(&mut child.max_children, parent.max_children);
    cap_budget_u8(
        &mut child.max_delegation_depth,
        parent
            .max_delegation_depth
            .map(|limit| limit.saturating_sub(1).max(1)),
    );
    Ok(child)
}

fn ensure_budget_remaining_u32(limit: Option<u32>, used: u32, field: &str) -> Result<(), AppError> {
    if limit.is_some_and(|limit| used >= limit) {
        return Err(AppError::InvalidState {
            current: format!("parent Agent budget {field} 已耗尽"),
            operation: "创建 child Agent".into(),
        });
    }
    Ok(())
}

fn ensure_budget_remaining_u64(limit: Option<u64>, used: u64, field: &str) -> Result<(), AppError> {
    if limit.is_some_and(|limit| used >= limit) {
        return Err(AppError::InvalidState {
            current: format!("parent Agent budget {field} 已耗尽"),
            operation: "创建 child Agent".into(),
        });
    }
    Ok(())
}

fn cap_budget_u32(child: &mut Option<u32>, parent: Option<u32>, used: u32) {
    if let Some(limit) = parent {
        let remaining = limit.saturating_sub(used).max(1);
        *child = Some(child.unwrap_or(remaining).min(remaining));
    }
}

fn cap_budget_u64(child: &mut Option<u64>, parent: Option<u64>, used: u64) {
    if let Some(limit) = parent {
        let remaining = limit.saturating_sub(used).max(1);
        *child = Some(child.unwrap_or(remaining).min(remaining));
    }
}

fn cap_budget_u16(child: &mut Option<u16>, parent: Option<u16>) {
    if let Some(limit) = parent {
        *child = Some(child.unwrap_or(limit).min(limit));
    }
}

fn cap_budget_u8(child: &mut Option<u8>, parent: Option<u8>) {
    if let Some(limit) = parent {
        *child = Some(child.unwrap_or(limit).min(limit));
    }
}

fn budget_usage_u32(value: &Value, key: &str) -> Result<u32, AppError> {
    u32::try_from(budget_usage_u64(value, key)?).map_err(|_| AppError::InvalidState {
        current: format!("Agent usage.{key} 超出 u32"),
        operation: "读取 Agent run budget usage".into(),
    })
}

fn budget_usage_u64(value: &Value, key: &str) -> Result<u64, AppError> {
    match value.get(key) {
        None => Ok(0),
        Some(value) => value.as_u64().ok_or_else(|| AppError::InvalidState {
            current: format!("Agent usage.{key} 不是非负整数"),
            operation: "读取 Agent run budget usage".into(),
        }),
    }
}
