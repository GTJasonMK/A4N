use super::*;

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
    pub(in crate::optimize) llm_error_label: &'a str,
    pub(in crate::optimize) parse_error_label: &'a str,
    pub(in crate::optimize) progress_current: u32,
    pub(in crate::optimize) progress_total: u32,
    pub(in crate::optimize) progress_message: &'a str,
    pub(in crate::optimize) release_operation: Option<&'a str>,
    pub(in crate::optimize) cancel_session_on_progress_error: bool,
    pub(in crate::optimize) workflow_run_id: i64,
}

pub(in crate::optimize) struct AgentLoopThinkObservationInput<'a> {
    pub(in crate::optimize) turn_id: i64,
    pub(in crate::optimize) session_id: i64,
    pub(in crate::optimize) step: usize,
    pub(in crate::optimize) decision: &'a AgentLoopDecision,
    pub(in crate::optimize) think_event_prefix: &'a str,
    pub(in crate::optimize) error_context: &'a str,
    pub(in crate::optimize) release_operation: Option<&'a str>,
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

pub(in crate::optimize) struct ExplicitAnalysisAgentLoopInput<'a, 'progress, 'stream> {
    pub(in crate::optimize) turn_id: i64,
    pub(in crate::optimize) session: optimization_session::Model,
    pub(in crate::optimize) analysis_prompt: String,
    pub(in crate::optimize) base_user_message: String,
    pub(in crate::optimize) analysis_context: Value,
    pub(in crate::optimize) workflow_run_id: i64,
    pub(in crate::optimize) progress: &'a mut Option<OptimizeProgress<'progress>>,
    pub(in crate::optimize) stream: &'a mut Option<OptimizeLlmStream<'stream>>,
}
