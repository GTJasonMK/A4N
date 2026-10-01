mod budget;
mod inputs;
mod status;

pub use budget::{
    AGENT_DEFAULT_MAX_STEPS, AGENT_DEFAULT_MAX_TOOLS, AgentBudget, agent_model_token_reservation,
};
pub use inputs::{
    AgentApprovalInput, AgentCompletionBlocker, AgentDelegationInput, AgentDelegationReviewInput,
    AgentEventInput, AgentExecutionInput, AgentInboxInput, AgentMemoryInput, AgentMessageInput,
    AgentThreadCreateInput, AgentTurnInput, AgentWorkflowOrigin,
};
pub use status::{
    AgentApprovalDecision, AgentApprovalStatus, AgentControlAction, AgentDelegationStatus,
    AgentDeliveryMode, AgentExecutionKind, AgentExecutionMode, AgentExecutionStatus,
    AgentInboxKind, AgentInboxStatus, AgentMemoryScopeKind, AgentMemoryStatus, AgentMessageRole,
    AgentMessageStatus, AgentRunStatus, AgentThreadKind, AgentThreadStatus, AgentToolCostClass,
    AgentToolExposure, AgentToolReplayPolicy, AgentToolReversibility, AgentToolRisk,
    AgentTurnStatus, AgentTurnTrigger, AgentWaitingReason,
};

pub const AGENT_PROTOCOL_NATIVE_TOOL_CALLING: &str = "native_tool_calling";
pub const AGENT_PROTOCOL_JSON_COMPAT: &str = "json_compat";

pub fn valid_agent_protocol(value: &str) -> bool {
    matches!(
        value.trim(),
        AGENT_PROTOCOL_NATIVE_TOOL_CALLING | AGENT_PROTOCOL_JSON_COMPAT
    )
}

#[cfg(test)]
mod tests {
    use super::{AgentBudget, AgentRunStatus};

    #[test]
    fn duration_budget_must_fit_wall_and_monotonic_clocks() {
        for limit in [u64::MAX, i64::MAX as u64, (i64::MAX / 1000) as u64] {
            let budget = AgentBudget {
                max_duration_seconds: Some(limit),
                ..Default::default()
            };
            assert!(
                budget
                    .validate()
                    .unwrap_err()
                    .contains("max_duration_seconds")
            );
        }
        for limit in [None, Some(1), Some(86_400)] {
            AgentBudget {
                max_duration_seconds: limit,
                ..Default::default()
            }
            .validate()
            .unwrap();
        }
    }

    #[test]
    fn recovery_required_is_terminal_but_waiting_is_not() {
        assert!(AgentRunStatus::RecoveryRequired.is_terminal());
        assert!(!AgentRunStatus::Waiting.is_terminal());
    }
}
