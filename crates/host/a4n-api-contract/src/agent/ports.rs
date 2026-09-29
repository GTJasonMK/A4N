use super::*;

/// Host-facing Agent thread mutation boundary.
#[async_trait::async_trait]
pub trait AgentThreadMutationApplicationPort: Send + Sync {
    async fn ensure_main_thread(
        &self,
        user_id: i64,
        input: EnsureMainAgentThreadInput,
    ) -> Result<AgentThreadView, a4n_error::AppError>;

    async fn create_thread(
        &self,
        user_id: i64,
        input: CreateAgentThreadInput,
    ) -> Result<AgentThreadView, a4n_error::AppError>;

    async fn fork_thread(
        &self,
        user_id: i64,
        input: ForkAgentThreadInput,
    ) -> Result<AgentThreadView, a4n_error::AppError>;

    async fn update_thread(
        &self,
        user_id: i64,
        input: UpdateAgentThreadInput,
    ) -> Result<AgentThreadView, a4n_error::AppError>;

    async fn archive_thread(
        &self,
        user_id: i64,
        input: ArchiveAgentThreadInput,
    ) -> Result<AgentThreadView, a4n_error::AppError>;
}

/// Host-facing Agent thread read boundary.
#[async_trait::async_trait]
pub trait AgentThreadReadApplicationPort: Send + Sync {
    async fn list_threads(
        &self,
        user_id: i64,
        input: ListAgentThreadsInput,
    ) -> Result<Vec<AgentThreadView>, a4n_error::AppError>;

    async fn show_thread(
        &self,
        user_id: i64,
        thread_id: &str,
    ) -> Result<AgentThreadSnapshot, a4n_error::AppError>;

    async fn list_messages(
        &self,
        user_id: i64,
        input: ListAgentMessagesInput,
    ) -> Result<Vec<AgentMessageView>, a4n_error::AppError>;
}

/// Host-facing Agent run mutation boundary.
#[async_trait::async_trait]
pub trait AgentRunMutationApplicationPort: Send + Sync {
    async fn send_message(
        &self,
        user_id: i64,
        input: SendAgentMessageInput,
        runtime_snapshot_required: bool,
        require_runtime_snapshot_for_new_run: bool,
    ) -> Result<AgentMessageAccepted, a4n_error::AppError>;

    async fn control_run(
        &self,
        user_id: i64,
        input: ControlAgentRunInput,
    ) -> Result<AgentRunView, a4n_error::AppError>;

    async fn send_message_to_child(
        &self,
        user_id: i64,
        input: SendAgentMessageToChildInput,
        runtime_snapshot_required: bool,
        require_runtime_snapshot_for_new_run: bool,
    ) -> Result<AgentMessageAccepted, a4n_error::AppError>;

    async fn control_child_agent(
        &self,
        user_id: i64,
        input: ControlChildAgentInput,
    ) -> Result<AgentRunView, a4n_error::AppError>;
}

/// Host-facing Agent run read boundary.
#[async_trait::async_trait]
pub trait AgentRunReadApplicationPort: Send + Sync {
    async fn show_run(
        &self,
        user_id: i64,
        run_id: &str,
    ) -> Result<AgentRunView, a4n_error::AppError>;
}

/// Host-facing Agent delegation boundary.
#[async_trait::async_trait]
pub trait AgentDelegationApplicationPort: Send + Sync {
    async fn spawn_agent(
        &self,
        user_id: i64,
        input: SpawnAgentInput,
    ) -> Result<AgentDelegationView, a4n_error::AppError>;

    async fn review_delegation(
        &self,
        user_id: i64,
        input: ReviewAgentDelegationInput,
        parent_run_id: Option<i64>,
    ) -> Result<AgentDelegationView, a4n_error::AppError>;
}

/// Host-facing Agent event read boundary.
#[async_trait::async_trait]
pub trait AgentEventReadApplicationPort: Send + Sync {
    async fn events(
        &self,
        user_id: i64,
        input: AgentEventsInput,
    ) -> Result<Vec<AgentEvent>, a4n_error::AppError>;
}

/// Host-facing Agent tool discovery boundary.
#[async_trait::async_trait]
pub trait AgentToolReadApplicationPort: Send + Sync {
    async fn search_tools(
        &self,
        user_id: i64,
        input: SearchAgentToolsInput,
    ) -> Result<AgentToolSearchResult, a4n_error::AppError>;

    async fn describe_tool(
        &self,
        user_id: i64,
        input: DescribeAgentToolInput,
    ) -> Result<AgentToolDescriptor, a4n_error::AppError>;
}

/// Host-facing Skill catalog and configuration boundary.
#[async_trait::async_trait]
pub trait AgentSkillApplicationPort: Send + Sync {
    async fn list_skill_settings(
        &self,
        user_id: i64,
    ) -> Result<AgentSkillSettingsView, a4n_error::AppError>;

    async fn update_skill_configuration(
        &self,
        user_id: i64,
        input: UpdateAgentSkillConfigurationInput,
    ) -> Result<AgentSkillProfileView, a4n_error::AppError>;

    async fn search_skills(
        &self,
        user_id: i64,
        capability: &str,
        input: SearchAgentSkillsInput,
    ) -> Result<AgentSkillSearchResult, a4n_error::AppError>;

    async fn load_skill(
        &self,
        user_id: i64,
        capability: &str,
        input: LoadAgentSkillInput,
    ) -> Result<AgentSkillContent, a4n_error::AppError>;
}

/// Host-facing Agent memory read boundary.
#[async_trait::async_trait]
pub trait AgentMemoryReadApplicationPort: Send + Sync {
    async fn list_memories(
        &self,
        user_id: i64,
        scope_kind: Option<AgentMemoryScopeKind>,
        scope_id: Option<String>,
    ) -> Result<Vec<AgentMemoryView>, a4n_error::AppError>;
}

/// Host-facing Agent memory mutation boundary.
#[async_trait::async_trait]
pub trait AgentMemoryMutationApplicationPort: Send + Sync {
    async fn create_memory(
        &self,
        user_id: i64,
        input: AgentMemoryInput,
    ) -> Result<AgentMemoryView, a4n_error::AppError>;

    async fn retract_memory(
        &self,
        user_id: i64,
        input: RetractAgentMemoryInput,
    ) -> Result<AgentMemoryView, a4n_error::AppError>;
}

/// Host-facing Agent approval mutation boundary.
#[async_trait::async_trait]
pub trait AgentApprovalApplicationPort: Send + Sync {
    async fn resolve_approval(
        &self,
        user_id: i64,
        input: ResolveAgentApprovalInput,
    ) -> Result<AgentApprovalView, a4n_error::AppError>;
}

/// Host-facing Agent resource authorization boundary.
#[async_trait::async_trait]
pub trait AgentResourceAccessApplicationPort: Send + Sync {
    async fn verify_chapters_in_project(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_ids: &[i64],
        source_chapter_ids: &[i64],
    ) -> Result<(), a4n_error::AppError>;
}
