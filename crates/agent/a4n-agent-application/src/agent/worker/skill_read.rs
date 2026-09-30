use super::super::AgentApplication;
use a4n_agent_runtime_contract::AgentWorkerSkillReadApplicationPort;
use a4n_api_contract::agent::{AgentSkillContent, AgentSkillSearchResult};
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentWorkerRunLeaseRequest, AgentWorkerSkillLoadRequest, AgentWorkerSkillSearchRequest,
};

impl AgentApplication {
    pub(crate) async fn search_skills_for_worker(
        &self,
        request: &AgentWorkerSkillSearchRequest,
    ) -> Result<AgentSkillSearchResult, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let claimed = self
            .worker_run_for_lease_request(&AgentWorkerRunLeaseRequest {
                run_id: request.run_id,
                lease: request.lease.clone(),
            })
            .await?;
        self.search_skills(
            claimed.thread.owner_user_id,
            &claimed.thread.profile_id,
            request.input.clone(),
        )
        .await
    }

    pub(crate) async fn load_skill_for_worker(
        &self,
        request: &AgentWorkerSkillLoadRequest,
    ) -> Result<AgentSkillContent, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let claimed = self
            .worker_run_for_lease_request(&AgentWorkerRunLeaseRequest {
                run_id: request.run_id,
                lease: request.lease.clone(),
            })
            .await?;
        self.load_skill(
            claimed.thread.owner_user_id,
            &claimed.thread.profile_id,
            request.input.clone(),
        )
        .await
    }
}

#[async_trait::async_trait]
impl AgentWorkerSkillReadApplicationPort for AgentApplication {
    async fn search_skills_for_worker(
        &self,
        request: &AgentWorkerSkillSearchRequest,
    ) -> Result<AgentSkillSearchResult, AppError> {
        AgentApplication::search_skills_for_worker(self, request).await
    }

    async fn load_skill_for_worker(
        &self,
        request: &AgentWorkerSkillLoadRequest,
    ) -> Result<AgentSkillContent, AppError> {
        AgentApplication::load_skill_for_worker(self, request).await
    }
}
