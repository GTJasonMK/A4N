use a4n_llm::provider::{ChatRequest, ChatResponse};
use serde_json::Value;

use super::super::super::AgentLoopDecision;

pub(super) struct PreparedAgentDecisionCall {
    pub(super) chat_request: ChatRequest,
    pub(super) call_key: String,
    pub(super) input_fingerprint_json: Value,
}

pub(super) struct AgentDecisionCallResult {
    pub(super) response: ChatResponse,
    pub(super) decision: AgentLoopDecision,
    pub(super) restored: bool,
}
