//! Agent tool read port。

use super::*;

#[async_trait::async_trait]
impl a4n_api_contract::agent::AgentToolReadApplicationPort for AgentApplication {
    async fn search_tools(
        &self,
        _user_id: i64,
        input: a4n_api_contract::agent::SearchAgentToolsInput,
    ) -> Result<a4n_api_contract::agent::AgentToolSearchResult, AppError> {
        AgentApplication::search_tools(self, input)
    }

    async fn describe_tool(
        &self,
        _user_id: i64,
        input: a4n_api_contract::agent::DescribeAgentToolInput,
    ) -> Result<a4n_api_contract::agent::AgentToolDescriptor, AppError> {
        AgentApplication::describe_tool(self, input)
    }
}
