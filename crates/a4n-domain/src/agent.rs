use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentThreadKind {
    #[default]
    Main,
    Subagent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentThreadStatus {
    Active,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentMessageRole {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentMessageStatus {
    Streaming,
    Complete,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentTurnTrigger {
    User,
    Delegation,
    Recovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentTurnStatus {
    Queued,
    Active,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunStatus {
    Queued,
    Running,
    Waiting,
    Pausing,
    Paused,
    Cancelling,
    Cancelled,
    Completed,
    Failed,
    Interrupted,
    RecoveryRequired,
}

impl AgentRunStatus {
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Cancelled
                | Self::Completed
                | Self::Failed
                | Self::Interrupted
                | Self::RecoveryRequired
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentWaitingReason {
    User,
    Approval,
    Child,
    External,
    Backoff,
    Budget,
    Reconcile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentExecutionKind {
    ModelCall,
    ToolCall,
    Wait,
    Delegation,
    Verification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentExecutionStatus {
    Prepared,
    AwaitingUser,
    Dispatching,
    Running,
    Succeeded,
    Failed,
    Rejected,
    Cancelled,
    OutcomeUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentDeliveryMode {
    #[default]
    Followup,
    Steer,
    Inject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentExecutionMode {
    #[default]
    Autonomous,
    ReviewWrites,
    AdviceOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentControlAction {
    Pause,
    Resume,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentInboxKind {
    Followup,
    Steer,
    Inject,
    Control,
    ChildResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentInboxStatus {
    Pending,
    Claimed,
    Consumed,
    Superseded,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentDelegationStatus {
    Pending,
    Running,
    AwaitingAcceptance,
    RevisionRequested,
    Accepted,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    Consumed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentApprovalDecision {
    Approve,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentMemoryScopeKind {
    User,
    Project,
    Thread,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentMemoryStatus {
    Proposed,
    Active,
    Superseded,
    Retracted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentToolExposure {
    Hidden,
    Discoverable,
    Preloaded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentToolRisk {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentToolReversibility {
    Reversible,
    Compensatable,
    Irreversible,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentToolReplayPolicy {
    Safe,
    Idempotent,
    ReconcileRequired,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentToolCostClass {
    None,
    Llm,
    Image,
    Video,
    ExternalUnknown,
}

#[cfg(test)]
mod tests {
    use super::AgentRunStatus;

    #[test]
    fn recovery_required_is_terminal_but_waiting_is_not() {
        assert!(AgentRunStatus::RecoveryRequired.is_terminal());
        assert!(!AgentRunStatus::Waiting.is_terminal());
    }
}
