use a4n_db::workflow;
use a4n_db_models::models::ai::optimization_session;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatMessage, ChatResponse};
use serde_json::{Value, json};

use crate::workflow::workflow_checkpoint_contract_app_error;

use super::super::messages::{
    materialize_optimization_issue_evidence, parse_content_optimization_issues,
    parse_optimization_issues,
};
use super::super::turns::{
    EVENT_KIND_ASSISTANT_UPDATE, EVENT_KIND_ERROR, EVENT_KIND_TOOL_CALL, EVENT_KIND_TOOL_RESULT,
    EVENT_STATUS_COMPLETED, EVENT_STATUS_FAILED, EVENT_STATUS_RUNNING,
};
use super::super::{OrchestratorContext, optimize_llm_call_input_fingerprint_json};
use super::{
    AGENT_LOOP_RUNAWAY_GUARD_STEPS, AgentLoopAction, AgentLoopDecision, agent_loop_request_payload,
    build_agent_loop_chat_request, failed_tool_observation, final_issues,
    final_response_from_issues, parse_agent_loop_decision_response, run_tool_arguments,
    thinking_observation, tool_observation,
};

mod analysis {
    mod execution;
    mod finalization;
}
mod contracts;
mod decision {
    mod checkpoint;
    mod execution;
}
mod observations {
    mod thought;
    mod tool_call;
    mod tool_result;
}

pub(in crate::optimize) use contracts::{
    AgentLoopDecisionRequest, AgentLoopExecutionError, AgentLoopGenerationContext,
    AgentLoopThinkObservationInput, ExplicitAnalysisAgentLoopInput, FailedAgentLoopToolInput,
    OptimizeAnalysisTurnExecution,
};
