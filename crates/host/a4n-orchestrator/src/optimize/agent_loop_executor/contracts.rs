use super::*;
use crate::generation::{
    checkpoint::GenerationCheckpointWriter,
    events::{GenerationEventPublisher, GenerationTaskSession},
};
use a4n_service_protocol::GenerationError;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub(in crate::optimize) struct AgentLoopGenerationContext<'a> {
    pub(in crate::optimize) publisher: &'a dyn GenerationEventPublisher,
    pub(in crate::optimize) cancellation: &'a CancellationToken,
    pub(in crate::optimize) checkpoint_writer: &'a GenerationCheckpointWriter,
    pub(in crate::optimize) session: &'a Arc<GenerationTaskSession>,
    pub(in crate::optimize) agent_max_output_tokens: Option<usize>,
}

#[derive(Debug)]
pub(in crate::optimize) enum AgentLoopExecutionError {
    App(AppError),
    Generation(GenerationError),
}

impl From<AppError> for AgentLoopExecutionError {
    fn from(error: AppError) -> Self {
        Self::App(error)
    }
}

impl From<GenerationError> for AgentLoopExecutionError {
    fn from(error: GenerationError) -> Self {
        Self::Generation(error)
    }
}

impl std::fmt::Display for AgentLoopExecutionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::App(error) => error.fmt(formatter),
            Self::Generation(error) => write!(formatter, "{}: {}", error.code, error.message),
        }
    }
}

impl std::error::Error for AgentLoopExecutionError {}

impl AgentLoopExecutionError {
    pub(in crate::optimize) fn is_cancelled(&self) -> bool {
        match self {
            Self::App(AppError::Cancelled) => true,
            Self::Generation(error) => error.code.as_str() == "generation_interrupted",
            Self::App(_) => false,
        }
    }
}

pub(in crate::optimize) struct AgentLoopDecisionRequest<'a> {
    pub(in crate::optimize) project_id: i64,
    pub(in crate::optimize) workflow: &'a str,
    pub(in crate::optimize) phase: &'a str,
    pub(in crate::optimize) system_prompt: &'a str,
    pub(in crate::optimize) base_user_message: &'a str,
    pub(in crate::optimize) observations: &'a [Value],
    pub(in crate::optimize) step: usize,
    pub(in crate::optimize) build_error_label: &'a str,
    pub(in crate::optimize) request_event_label: &'a str,
    pub(in crate::optimize) parse_error_label: &'a str,
    pub(in crate::optimize) validate_final: &'a (dyn Fn(&Value) -> Result<(), AppError> + Sync),
    pub(in crate::optimize) workflow_run_id: i64,
    pub(in crate::optimize) generation: &'a AgentLoopGenerationContext<'a>,
}

pub(in crate::optimize) struct AgentLoopThinkObservationInput<'a> {
    pub(in crate::optimize) turn_id: i64,
    pub(in crate::optimize) session_id: i64,
    pub(in crate::optimize) step: usize,
    pub(in crate::optimize) decision: &'a AgentLoopDecision,
    pub(in crate::optimize) think_event_prefix: &'a str,
    pub(in crate::optimize) error_context: &'a str,
}

pub(in crate::optimize) struct FailedAgentLoopToolInput<'a> {
    pub(in crate::optimize) turn_id: i64,
    pub(in crate::optimize) session_id: i64,
    pub(in crate::optimize) step: usize,
    pub(in crate::optimize) tool_name: &'a str,
    pub(in crate::optimize) tool_arguments: Value,
    pub(in crate::optimize) err: &'a AppError,
    pub(in crate::optimize) tool_event_prefix: &'a str,
}

pub(in crate::optimize) struct ExplicitAnalysisAgentLoopInput<'a> {
    pub(in crate::optimize) turn_id: i64,
    pub(in crate::optimize) session: optimization_session::Model,
    pub(in crate::optimize) analysis_prompt: String,
    pub(in crate::optimize) base_user_message: String,
    pub(in crate::optimize) analysis_context: Value,
    pub(in crate::optimize) workflow_run_id: i64,
    pub(in crate::optimize) generation: &'a AgentLoopGenerationContext<'a>,
}

pub(in crate::optimize) struct OptimizeAnalysisTurnExecution {
    pub(in crate::optimize) turn_id: i64,
    pub(in crate::optimize) assistant_response: String,
    pub(in crate::optimize) canonical_issues: Value,
}
