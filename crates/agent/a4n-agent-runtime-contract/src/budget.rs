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
    budget.validate().map_err(AppError::InvalidArgument)
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

pub fn child_budget_for_delegation(
    mut child: AgentBudget,
    parent: &AgentBudget,
) -> Result<AgentBudget, AppError> {
    validate_agent_budget(&child)?;
    // Consumable quotas stay on their owning Run. DB admission intersects the
    // whole ancestry, so a parent extension must not leave a frozen child copy.
    cap_budget_u16(&mut child.max_parallel_tools, parent.max_parallel_tools);
    cap_budget_u16(&mut child.max_children, parent.max_children);
    // Depth is absolute from the root Run (depth zero), including in children.
    cap_budget_u8(&mut child.max_delegation_depth, parent.max_delegation_depth);
    Ok(child)
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
