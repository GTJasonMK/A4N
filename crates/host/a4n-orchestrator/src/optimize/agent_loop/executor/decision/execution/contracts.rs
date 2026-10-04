use a4n_agent_runtime_contract::AgentExecutionSnapshot;
use a4n_llm_provider_contract::provider::{ChatRequest, ChatResponse};
use serde_json::Value;

use super::super::super::{AgentLoopDecision, ValidatedAgentLoopAction};

pub(super) struct PreparedAgentDecisionCall {
    pub(super) chat_request: ChatRequest,
    pub(super) call_key: String,
    pub(super) input_fingerprint_json: Value,
    pub(super) execution: Option<AgentExecutionSnapshot>,
}

pub(super) struct AgentDecisionCallResult<T> {
    pub(super) response: ChatResponse,
    pub(super) decision: AgentLoopDecision,
    pub(super) action: ValidatedAgentLoopAction<T>,
    pub(super) restored: bool,
}
