use a4n_llm_provider_contract::provider::{ChatRequest, ChatResponse};
use serde_json::Value;

use super::super::super::AgentLoopDecision;

pub(super) struct PreparedAgentDecisionCall {
    pub(super) chat_request: ChatRequest,
    pub(super) call_key: String,
    pub(super) input_fingerprint_json: Value,
    pub(super) execution: Option<a4n_db_models::models::agent::execution::Model>,
}

pub(super) struct AgentDecisionCallResult {
    pub(super) response: ChatResponse,
    pub(super) decision: AgentLoopDecision,
    pub(super) restored: bool,
}
