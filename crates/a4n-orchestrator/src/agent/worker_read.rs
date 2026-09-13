use super::{AgentApplication, AgentRunContext};
use a4n_api_contract::agent::{
    AgentExecutionResultPage, AgentMemorySearchResult, ReadAgentExecutionResultInput,
    ReadAgentMemoryInput,
};
use a4n_error::AppError;

/// Runtime-facing boundary for the two read-only Agent tools available to a
/// claimed Run.
///
/// Thread ownership, scope filtering and result pagination remain in the
/// Agent owner; Runtime only supplies the durable context and typed input.
#[async_trait::async_trait]
pub trait AgentWorkerReadApplicationPort: Send + Sync {
    async fn read_memory_for_run(
        &self,
        context: &AgentRunContext,
        input: ReadAgentMemoryInput,
    ) -> Result<AgentMemorySearchResult, AppError>;

    async fn read_execution_result_for_run(
        &self,
        context: &AgentRunContext,
        input: ReadAgentExecutionResultInput,
    ) -> Result<AgentExecutionResultPage, AppError>;
}

#[async_trait::async_trait]
impl AgentWorkerReadApplicationPort for AgentApplication {
    async fn read_memory_for_run(
        &self,
        context: &AgentRunContext,
        input: ReadAgentMemoryInput,
    ) -> Result<AgentMemorySearchResult, AppError> {
        AgentApplication::read_memory_for_run(self, context, input).await
    }

    async fn read_execution_result_for_run(
        &self,
        context: &AgentRunContext,
        input: ReadAgentExecutionResultInput,
    ) -> Result<AgentExecutionResultPage, AppError> {
        AgentApplication::read_execution_result_for_run(self, context, input).await
    }
}
