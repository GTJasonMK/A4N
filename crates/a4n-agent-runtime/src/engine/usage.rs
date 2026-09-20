use super::*;

pub(crate) async fn persist_usage(
    state: &AgentRuntimeState,
    claimed: &AgentWorkerRun,
    usage: &Value,
    steps: u32,
    model_calls: u32,
    tool_calls: u32,
    tokens: u64,
) -> Result<(), AppError> {
    let mut usage = usage.clone();
    usage["steps_used"] = json!(steps);
    usage["model_calls_used"] = json!(model_calls);
    usage["tool_calls_used"] = json!(tool_calls);
    usage["tokens_used"] = json!(tokens);
    state.update_agent_worker_usage(claimed, usage).await?;
    Ok(())
}

#[derive(Clone, Copy)]
pub(crate) struct UsageCounters {
    pub(crate) steps: u32,
    pub(crate) model_calls: u32,
    pub(crate) tool_calls: u32,
    pub(crate) tokens: u64,
}

pub(crate) async fn record_provider_failure(
    state: &AgentRuntimeState,
    claimed: &AgentWorkerRun,
    usage: &mut Value,
    counters: UsageCounters,
    error: &AppError,
) -> Result<(), AppError> {
    if let Some(class) = a4n_agent_runtime_contract::provider_failure_class(error) {
        usage["failure_class"] = json!(class);
        persist_usage(
            state,
            claimed,
            usage,
            counters.steps,
            counters.model_calls,
            counters.tool_calls,
            counters.tokens,
        )
        .await?;
    }
    Ok(())
}
