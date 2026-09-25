use super::*;

pub(crate) async fn persist_usage(
    state: &AgentRuntimeState,
    claimed: &AgentWorkerRun,
    usage: &Value,
) -> Result<(), AppError> {
    state
        .update_agent_worker_usage(claimed, usage.clone())
        .await?;
    Ok(())
}

pub(crate) async fn record_provider_failure(
    state: &AgentRuntimeState,
    claimed: &AgentWorkerRun,
    usage: &mut Value,
    error: &AppError,
) -> Result<(), AppError> {
    if let Some(class) = a4n_agent_runtime_contract::provider_failure_class(error) {
        usage["failure_class"] = json!(class);
        persist_usage(state, claimed, usage).await?;
    }
    Ok(())
}
