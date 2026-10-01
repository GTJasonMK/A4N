use super::*;

#[async_trait::async_trait]
impl AgentToolReadApplicationPort for AgentWorkerClient {
    async fn search_tools(
        &self,
        user_id: i64,
        input: SearchAgentToolsInput,
    ) -> Result<AgentToolSearchResult, AppError> {
        self.command(user_id, AGENT_TOOL_SEARCH_OPERATION, input)
            .await
    }

    async fn describe_tool(
        &self,
        user_id: i64,
        input: DescribeAgentToolInput,
    ) -> Result<AgentToolDescriptor, AppError> {
        self.command(user_id, AGENT_TOOL_DESCRIBE_OPERATION, input)
            .await
    }
}
