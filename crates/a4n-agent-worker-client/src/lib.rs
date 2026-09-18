//! Typed Host adapter for the Agent Worker user-facing thread API.

use a4n_agent_runtime_contract::{
    AGENT_APPROVAL_RESOLVE_OPERATION, AGENT_DELEGATION_REVIEW_OPERATION,
    AGENT_DELEGATION_SPAWN_OPERATION, AGENT_EVENT_LIST_OPERATION, AGENT_MEMORY_CREATE_OPERATION,
    AGENT_MEMORY_LIST_OPERATION, AGENT_MEMORY_RETRACT_OPERATION, AGENT_RUN_SHOW_OPERATION,
    AGENT_THREAD_ARCHIVE_OPERATION, AGENT_THREAD_CREATE_OPERATION,
    AGENT_THREAD_ENSURE_MAIN_OPERATION, AGENT_THREAD_FORK_OPERATION, AGENT_THREAD_LIST_OPERATION,
    AGENT_THREAD_MESSAGES_OPERATION, AGENT_THREAD_SHOW_OPERATION, AGENT_THREAD_UPDATE_OPERATION,
    AGENT_WORKER_ID, AgentDelegationReviewCommand, AgentMemoryListInput,
};
use a4n_api_contract::agent::{
    AgentApprovalApplicationPort, AgentApprovalView, AgentDelegationApplicationPort,
    AgentDelegationView, AgentEvent, AgentEventReadApplicationPort, AgentMemoryInput,
    AgentMemoryMutationApplicationPort, AgentMemoryReadApplicationPort, AgentMemoryScopeKind,
    AgentMemoryView, AgentMessageView, AgentRunReadApplicationPort, AgentRunView,
    AgentThreadMutationApplicationPort, AgentThreadReadApplicationPort, AgentThreadSnapshot,
    AgentThreadView, ArchiveAgentThreadInput, CreateAgentThreadInput, EnsureMainAgentThreadInput,
    ForkAgentThreadInput, ListAgentMessagesInput, ListAgentThreadsInput, ResolveAgentApprovalInput,
    RetractAgentMemoryInput, ReviewAgentDelegationInput, SpawnAgentInput, UpdateAgentThreadInput,
};
use a4n_error::AppError;
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47650";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

#[derive(Clone)]
pub struct AgentWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for AgentWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
}

impl AgentWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, AGENT_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("A4N_AGENT_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                std::env::var("A4N_AGENT_WORKER_ADDR")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| format!("http://{value}"))
            })
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_AGENT_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        Self::new(url, token)
    }

    async fn command<I, O>(&self, user_id: i64, operation: &str, payload: I) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        if user_id <= 0 {
            return Err(AppError::InvalidArgument("user_id 必须大于 0".into()));
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: user_id.to_string(),
            },
            operation_id: operation.to_string(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id: None,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: AGENT_WORKER_ID.into(),
                epoch: 1,
                fence_token: CONTROL_LEASE_FENCE.into(),
                expires_at: CONTROL_LEASE_EXPIRY.into(),
            },
            payload: serde_json::to_value(payload)?,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation, request)
            .await
            .map_err(worker_client_error)?;
        match response.status {
            WorkerOperationStatus::Completed => response.payload.ok_or_else(|| {
                AppError::ConfigError(format!("agent worker 完成响应缺少 payload: {operation}"))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation,
            )),
            status => Err(AppError::ConfigError(format!(
                "agent worker 返回未完成状态: operation={operation}, status={status:?}"
            ))),
        }
    }
}

#[async_trait::async_trait]
impl AgentThreadMutationApplicationPort for AgentWorkerClient {
    async fn ensure_main_thread(
        &self,
        user_id: i64,
        input: EnsureMainAgentThreadInput,
    ) -> Result<AgentThreadView, AppError> {
        self.command(user_id, AGENT_THREAD_ENSURE_MAIN_OPERATION, input)
            .await
    }

    async fn create_thread(
        &self,
        user_id: i64,
        input: CreateAgentThreadInput,
    ) -> Result<AgentThreadView, AppError> {
        self.command(user_id, AGENT_THREAD_CREATE_OPERATION, input)
            .await
    }

    async fn fork_thread(
        &self,
        user_id: i64,
        input: ForkAgentThreadInput,
    ) -> Result<AgentThreadView, AppError> {
        self.command(user_id, AGENT_THREAD_FORK_OPERATION, input)
            .await
    }

    async fn update_thread(
        &self,
        user_id: i64,
        input: UpdateAgentThreadInput,
    ) -> Result<AgentThreadView, AppError> {
        self.command(user_id, AGENT_THREAD_UPDATE_OPERATION, input)
            .await
    }

    async fn archive_thread(
        &self,
        user_id: i64,
        input: ArchiveAgentThreadInput,
    ) -> Result<AgentThreadView, AppError> {
        self.command(user_id, AGENT_THREAD_ARCHIVE_OPERATION, input)
            .await
    }
}

#[async_trait::async_trait]
impl AgentThreadReadApplicationPort for AgentWorkerClient {
    async fn list_threads(
        &self,
        user_id: i64,
        input: ListAgentThreadsInput,
    ) -> Result<Vec<AgentThreadView>, AppError> {
        self.command(user_id, AGENT_THREAD_LIST_OPERATION, input)
            .await
    }

    async fn show_thread(
        &self,
        user_id: i64,
        thread_id: &str,
    ) -> Result<AgentThreadSnapshot, AppError> {
        self.command(user_id, AGENT_THREAD_SHOW_OPERATION, thread_id.to_string())
            .await
    }

    async fn list_messages(
        &self,
        user_id: i64,
        input: ListAgentMessagesInput,
    ) -> Result<Vec<AgentMessageView>, AppError> {
        self.command(user_id, AGENT_THREAD_MESSAGES_OPERATION, input)
            .await
    }
}

#[async_trait::async_trait]
impl AgentRunReadApplicationPort for AgentWorkerClient {
    async fn show_run(&self, user_id: i64, run_id: &str) -> Result<AgentRunView, AppError> {
        self.command(user_id, AGENT_RUN_SHOW_OPERATION, run_id.to_string())
            .await
    }
}

#[async_trait::async_trait]
impl AgentEventReadApplicationPort for AgentWorkerClient {
    async fn events(
        &self,
        user_id: i64,
        input: a4n_api_contract::agent::AgentEventsInput,
    ) -> Result<Vec<AgentEvent>, AppError> {
        self.command(user_id, AGENT_EVENT_LIST_OPERATION, input)
            .await
    }
}

#[async_trait::async_trait]
impl AgentMemoryReadApplicationPort for AgentWorkerClient {
    async fn list_memories(
        &self,
        user_id: i64,
        scope_kind: Option<AgentMemoryScopeKind>,
        scope_id: Option<String>,
    ) -> Result<Vec<AgentMemoryView>, AppError> {
        self.command(
            user_id,
            AGENT_MEMORY_LIST_OPERATION,
            AgentMemoryListInput {
                scope_kind,
                scope_id,
            },
        )
        .await
    }
}

#[async_trait::async_trait]
impl AgentMemoryMutationApplicationPort for AgentWorkerClient {
    async fn create_memory(
        &self,
        user_id: i64,
        input: AgentMemoryInput,
    ) -> Result<AgentMemoryView, AppError> {
        self.command(user_id, AGENT_MEMORY_CREATE_OPERATION, input)
            .await
    }

    async fn retract_memory(
        &self,
        user_id: i64,
        input: RetractAgentMemoryInput,
    ) -> Result<AgentMemoryView, AppError> {
        self.command(user_id, AGENT_MEMORY_RETRACT_OPERATION, input)
            .await
    }
}

#[async_trait::async_trait]
impl AgentApprovalApplicationPort for AgentWorkerClient {
    async fn resolve_approval(
        &self,
        user_id: i64,
        input: ResolveAgentApprovalInput,
    ) -> Result<AgentApprovalView, AppError> {
        self.command(user_id, AGENT_APPROVAL_RESOLVE_OPERATION, input)
            .await
    }
}

#[async_trait::async_trait]
impl AgentDelegationApplicationPort for AgentWorkerClient {
    async fn spawn_agent(
        &self,
        user_id: i64,
        input: SpawnAgentInput,
    ) -> Result<AgentDelegationView, AppError> {
        self.command(user_id, AGENT_DELEGATION_SPAWN_OPERATION, input)
            .await
    }

    async fn review_delegation(
        &self,
        user_id: i64,
        input: ReviewAgentDelegationInput,
        parent_run_id: Option<i64>,
    ) -> Result<AgentDelegationView, AppError> {
        self.command(
            user_id,
            AGENT_DELEGATION_REVIEW_OPERATION,
            AgentDelegationReviewCommand {
                input,
                parent_run_id,
            },
        )
        .await
    }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    AppError::ConfigError(format!("agent worker 请求失败: {error}"))
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    match error.code {
        WorkerErrorCode::InvalidRequest => AppError::InvalidArgument(error.message.clone()),
        WorkerErrorCode::Unauthorized => AppError::Forbidden(error.message.clone()),
        WorkerErrorCode::NotFound => AppError::NotFound(error.message.clone()),
        WorkerErrorCode::Conflict => AppError::InvalidState {
            current: error.message.clone(),
            operation: operation.to_string(),
        },
        _ => AppError::ConfigError(format!("agent worker {operation} 失败: {}", error.message)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client =
            AgentWorkerClient::new("https://worker.example.test", Some("worker-token".into()));
        let debug = format!("{client:?}");
        assert!(!debug.contains("worker.example.test"));
        assert!(!debug.contains("worker-token"));
    }
}
