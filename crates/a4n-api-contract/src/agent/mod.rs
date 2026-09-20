//! 全局 Agent 控制平面合同。
//!
//! Agent 只负责会话、编排、工具调用和委派；正文、画布、Workflow、Generation
//! 等领域数据仍由各自的合同和事实源负责。本模块因此只保存引用，不复制领域实体。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::runtime_config::RuntimeConfigSnapshot;

pub use a4n_domain::agent::{
    AGENT_DEFAULT_MAX_STEPS, AGENT_DEFAULT_MAX_TOOLS, AGENT_PROTOCOL_JSON_COMPAT,
    AGENT_PROTOCOL_NATIVE_TOOL_CALLING, AgentApprovalDecision, AgentApprovalStatus, AgentBudget,
    AgentControlAction, AgentDelegationStatus, AgentDeliveryMode, AgentExecutionKind,
    AgentExecutionMode, AgentExecutionStatus, AgentInboxKind, AgentInboxStatus,
    AgentMemoryScopeKind, AgentMemoryStatus, AgentMessageRole, AgentMessageStatus, AgentRunStatus,
    AgentThreadKind, AgentThreadStatus, AgentToolCostClass, AgentToolExposure,
    AgentToolReplayPolicy, AgentToolReversibility, AgentToolRisk, AgentTurnStatus,
    AgentTurnTrigger, AgentWaitingReason,
};

mod catalog;
mod inputs;
mod ports;
#[cfg(test)]
mod tests;
mod types;

pub use catalog::*;
pub use inputs::*;
pub use ports::*;
pub use types::*;
