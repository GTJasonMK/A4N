//! Agent approval与资源访问 ports。

use super::*;

#[async_trait::async_trait]
impl a4n_api_contract::agent::AgentApprovalApplicationPort for AgentApplication {
    async fn resolve_approval(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::ResolveAgentApprovalInput,
    ) -> Result<a4n_api_contract::agent::AgentApprovalView, AppError> {
        AgentApplication::resolve_approval(self, user_id, input).await
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::agent::AgentResourceAccessApplicationPort for AgentApplication {
    async fn verify_chapters_in_project(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_ids: &[i64],
        source_chapter_ids: &[i64],
    ) -> Result<(), AppError> {
        AgentApplication::verify_chapters_in_project(
            self,
            user_id,
            project_id,
            chapter_ids,
            source_chapter_ids,
        )
        .await
    }
}
