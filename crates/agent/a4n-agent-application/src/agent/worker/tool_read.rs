use super::super::AgentApplication;
use a4n_agent_runtime_contract::AgentWorkerToolReadApplicationPort;
use a4n_api_contract::agent::{
    AgentToolDescriptor, AgentToolSearchResult, DescribeAgentToolInput, SearchAgentToolsInput,
};
use a4n_error::AppError;

#[async_trait::async_trait]
impl AgentWorkerToolReadApplicationPort for AgentApplication {
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
}
