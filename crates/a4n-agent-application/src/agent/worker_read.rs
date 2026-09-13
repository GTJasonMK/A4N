use super::{AgentApplication, AgentRunContext};
use a4n_api_contract::agent::{
    AgentExecutionResultPage, AgentMemorySearchResult, AgentToolDescriptor, AgentToolSearchResult,
    DescribeAgentToolInput, ReadAgentExecutionResultInput, ReadAgentMemoryInput,
    SearchAgentToolsInput,
};
use a4n_error::AppError;
use a4n_worker_contract::{AgentWorkerExecutionResultRequest, AgentWorkerMemoryRequest};

/// Runtime-facing boundary for the two read-only Agent tools available to a
/// claimed Run.
///
/// Thread ownership, scope filtering and result pagination remain in the
/// Agent owner; Runtime only supplies the durable context and typed input.
#[async_trait::async_trait]
pub trait AgentWorkerReadApplicationPort: Send + Sync {
    fn search_tools_for_worker(
        &self,
        input: SearchAgentToolsInput,
    ) -> Result<AgentToolSearchResult, AppError>;

    fn describe_tool_for_worker(
        &self,
        input: DescribeAgentToolInput,
    ) -> Result<AgentToolDescriptor, AppError>;

    async fn verify_chapters_in_project_for_worker(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_ids: &[i64],
    ) -> Result<(), AppError>;

    async fn read_memory_for_run(
        &self,
        context: &AgentRunContext,
        input: ReadAgentMemoryInput,
    ) -> Result<AgentMemorySearchResult, AppError>;

    async fn read_memory_request(
        &self,
        request: &AgentWorkerMemoryRequest,
    ) -> Result<AgentMemorySearchResult, AppError>;

    async fn read_execution_result_for_run(
        &self,
        context: &AgentRunContext,
        input: ReadAgentExecutionResultInput,
    ) -> Result<AgentExecutionResultPage, AppError>;

    async fn read_execution_result_request(
        &self,
        request: &AgentWorkerExecutionResultRequest,
    ) -> Result<AgentExecutionResultPage, AppError>;
}

#[async_trait::async_trait]
impl AgentWorkerReadApplicationPort for AgentApplication {
    fn search_tools_for_worker(
        &self,
        input: SearchAgentToolsInput,
    ) -> Result<AgentToolSearchResult, AppError> {
        AgentApplication::search_tools(self, input)
    }

    fn describe_tool_for_worker(
        &self,
        input: DescribeAgentToolInput,
    ) -> Result<AgentToolDescriptor, AppError> {
        AgentApplication::describe_tool(self, input)
    }

    async fn verify_chapters_in_project_for_worker(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_ids: &[i64],
    ) -> Result<(), AppError> {
        AgentApplication::verify_chapters_in_project(self, user_id, project_id, chapter_ids).await
    }

    async fn read_memory_for_run(
        &self,
        context: &AgentRunContext,
        input: ReadAgentMemoryInput,
    ) -> Result<AgentMemorySearchResult, AppError> {
        AgentApplication::read_memory_for_run(self, context, input).await
    }

    async fn read_memory_request(
        &self,
        request: &AgentWorkerMemoryRequest,
    ) -> Result<AgentMemorySearchResult, AppError> {
        AgentApplication::read_memory_request(self, request).await
    }

    async fn read_execution_result_for_run(
        &self,
        context: &AgentRunContext,
        input: ReadAgentExecutionResultInput,
    ) -> Result<AgentExecutionResultPage, AppError> {
        AgentApplication::read_execution_result_for_run(self, context, input).await
    }

    async fn read_execution_result_request(
        &self,
        request: &AgentWorkerExecutionResultRequest,
    ) -> Result<AgentExecutionResultPage, AppError> {
        AgentApplication::read_execution_result_request(self, request).await
    }
}
