use super::super::AgentMessageSnapshot;
use super::super::AgentRunContext;
use super::super::{
    AgentApplication, AgentThreadSnapshot, AgentWorkerRun, EnsureCanonicalSubagentThreadInput,
    WorkerDelegationInput, WorkerDelegationResult, thread_snapshot,
};
use a4n_agent_runtime_contract::AgentWorkerDelegationApplicationPort;
use a4n_agent_runtime_contract::AgentWorkerHistoryApplicationPort;
use a4n_agent_runtime_contract::AgentWorkerReadApplicationPort;
use a4n_api_contract::agent::{AgentDelegationView, AgentScope, ReviewAgentDelegationInput};
use a4n_api_contract::agent::{
    AgentExecutionResultPage, AgentMemorySearchResult, AgentToolDescriptor, AgentToolSearchResult,
    DescribeAgentToolInput, ReadAgentExecutionResultInput, ReadAgentMemoryInput,
    SearchAgentToolsInput,
};
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentWorkerDelegationReviewRequest, AgentWorkerDelegationReviewSnapshotWire,
    AgentWorkerDelegationStartRequest, AgentWorkerDelegationStartSnapshotWire,
    AgentWorkerDelegationThreadRequest, AgentWorkerDelegationThreadSnapshotWire,
    AgentWorkerEventRequest,
};
use a4n_worker_contract::{
    AgentWorkerExecutionResultRequest, AgentWorkerMemoryRequest, AgentWorkerSkillLoadRequest,
    AgentWorkerSkillSearchRequest,
};
use a4n_worker_contract::{AgentWorkerHistoryRequest, AgentWorkerHistorySnapshotWire};
use serde_json::Value;

#[async_trait::async_trait]
impl AgentWorkerDelegationApplicationPort for AgentApplication {
    async fn review_delegation(
        &self,
        user_id: i64,
        input: ReviewAgentDelegationInput,
        parent_run_id: Option<i64>,
    ) -> Result<AgentDelegationView, AppError> {
        AgentApplication::review_delegation(self, user_id, input, parent_run_id).await
    }

    async fn review_delegation_request(
        &self,
        request: &AgentWorkerDelegationReviewRequest,
    ) -> Result<AgentWorkerDelegationReviewSnapshotWire, AppError> {
        AgentApplication::review_delegation_request(self, request).await
    }

    async fn ensure_canonical_subagent_thread_request(
        &self,
        request: &AgentWorkerDelegationThreadRequest,
    ) -> Result<AgentWorkerDelegationThreadSnapshotWire, AppError> {
        AgentApplication::ensure_canonical_subagent_thread_request(self, request).await
    }

    async fn append_worker_update_request(
        &self,
        request: &AgentWorkerEventRequest,
    ) -> Result<(), AppError> {
        AgentApplication::append_worker_update_request(self, request).await
    }

    async fn create_child_run_request(
        &self,
        request: &AgentWorkerDelegationStartRequest,
    ) -> Result<AgentWorkerDelegationStartSnapshotWire, AppError> {
        AgentApplication::create_child_run_request(self, request).await
    }

    fn inherit_subagent_allowlists(
        &self,
        parent: &AgentScope,
        child: &mut AgentScope,
    ) -> Result<(), AppError> {
        super::super::inherit_subagent_allowlists(parent, child)
    }

    async fn ensure_canonical_subagent_thread(
        &self,
        user_id: i64,
        input: EnsureCanonicalSubagentThreadInput,
    ) -> Result<AgentThreadSnapshot, AppError> {
        AgentApplication::ensure_canonical_subagent_thread(self, user_id, input)
            .await
            .and_then(thread_snapshot)
    }

    async fn append_worker_update(
        &self,
        claimed: &AgentWorkerRun,
        kind: &str,
        status: &str,
        public_text: String,
        payload_json: Option<Value>,
    ) -> Result<(), AppError> {
        AgentApplication::append_worker_update(
            self,
            claimed,
            kind,
            status,
            public_text,
            payload_json,
        )
        .await
        .map(|_| ())
    }

    async fn create_child_run(
        &self,
        claimed: &AgentWorkerRun,
        input: WorkerDelegationInput,
    ) -> Result<WorkerDelegationResult, AppError> {
        AgentApplication::create_child_run(self, claimed, input).await
    }
}

#[async_trait::async_trait]
impl AgentWorkerHistoryApplicationPort for AgentApplication {
    async fn user_messages_for_run(
        &self,
        run_id: i64,
    ) -> Result<Vec<AgentMessageSnapshot>, AppError> {
        AgentApplication::user_messages_for_run(self, run_id).await
    }

    async fn latest_model_input_message_id(&self, run_id: i64) -> Result<Option<i64>, AppError> {
        AgentApplication::latest_model_input_message_id(self, run_id).await
    }

    async fn worker_history_request(
        &self,
        request: &AgentWorkerHistoryRequest,
    ) -> Result<AgentWorkerHistorySnapshotWire, AppError> {
        AgentApplication::worker_history_request(self, request).await
    }
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
