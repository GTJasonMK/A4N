use super::{AgentApplication, AgentRunContext};
use a4n_agent_runtime_contract::AgentWorkerReadApplicationPort;
use a4n_api_contract::agent::{
    AgentExecutionResultPage, AgentMemorySearchResult, AgentToolDescriptor, AgentToolSearchResult,
    DescribeAgentToolInput, ReadAgentExecutionResultInput, ReadAgentMemoryInput,
    SearchAgentToolsInput,
};
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentWorkerExecutionResultRequest, AgentWorkerMemoryRequest, AgentWorkerSkillLoadRequest,
    AgentWorkerSkillSearchRequest,
};

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

    async fn search_skills_for_worker(
        &self,
        request: &AgentWorkerSkillSearchRequest,
    ) -> Result<a4n_api_contract::agent::AgentSkillSearchResult, AppError> {
        AgentApplication::search_skills_for_worker(self, request).await
    }

    async fn load_skill_for_worker(
        &self,
        request: &AgentWorkerSkillLoadRequest,
    ) -> Result<a4n_api_contract::agent::AgentSkillContent, AppError> {
        AgentApplication::load_skill_for_worker(self, request).await
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
