use super::*;

#[async_trait::async_trait]
impl AgentSkillApplicationPort for AgentWorkerClient {
    async fn list_skill_settings(&self, user_id: i64) -> Result<AgentSkillSettingsView, AppError> {
        self.command(user_id, AGENT_SKILL_SETTINGS_LIST_OPERATION, ())
            .await
    }

    async fn update_skill_configuration(
        &self,
        user_id: i64,
        input: UpdateAgentSkillConfigurationInput,
    ) -> Result<AgentSkillProfileView, AppError> {
        self.command(user_id, AGENT_SKILL_SETTINGS_UPDATE_OPERATION, input)
            .await
    }

    async fn search_skills(
        &self,
        user_id: i64,
        capability: &str,
        input: SearchAgentSkillsInput,
    ) -> Result<AgentSkillSearchResult, AppError> {
        self.command(
            user_id,
            AGENT_SKILL_SEARCH_OPERATION,
            AgentSkillCapabilityCommand {
                capability: capability.to_string(),
                input,
            },
        )
        .await
    }

    async fn load_skill(
        &self,
        user_id: i64,
        capability: &str,
        input: LoadAgentSkillInput,
    ) -> Result<AgentSkillContent, AppError> {
        self.command(
            user_id,
            AGENT_SKILL_LOAD_OPERATION,
            AgentSkillCapabilityCommand {
                capability: capability.to_string(),
                input,
            },
        )
        .await
    }
}
