use super::super::{AgentApplication, AgentRunContext};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerMemoryReadApplicationPort;
use a4n_api_contract::agent::{AgentMemorySearchResult, ReadAgentMemoryInput};
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerMemoryRequest;

impl AgentApplication {
    pub(crate) async fn read_memory_for_run(
        &self,
        context: &AgentRunContext,
        input: ReadAgentMemoryInput,
    ) -> Result<AgentMemorySearchResult, AppError> {
        let thread_row = &context.thread;
        let limit = bounded_limit(input.limit, 20, 100, "limit")? as usize;
        let query = input
            .query
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_lowercase();
        let mut memories =
            agent::list_memory(self.connection(), thread_row.owner_user_id, None, None)
                .await
                .map_err(agent_db_error_to_app)?;
        memories.retain(|row| {
            row.status == "active"
                && match row.scope_kind.as_str() {
                    "user" => row.scope_key == thread_row.owner_user_id.to_string(),
                    "project" => thread_row
                        .project_id
                        .is_some_and(|project_id| row.scope_key == project_id.to_string()),
                    "thread" => row.scope_key == thread_row.id.to_string(),
                    _ => false,
                }
        });
        memories.sort_by(|left, right| {
            (&left.scope_kind, &left.scope_key, &left.memory_key)
                .cmp(&(&right.scope_kind, &right.scope_key, &right.memory_key))
                .then_with(|| right.revision.cmp(&left.revision))
        });
        memories.dedup_by(|left, right| {
            left.scope_kind == right.scope_kind
                && left.scope_key == right.scope_key
                && left.memory_key == right.memory_key
        });
        memories.retain(|row| {
            row.memory_key.to_lowercase().contains(&query)
                || row.content.to_lowercase().contains(&query)
        });
        let total = memories.len();
        memories.truncate(limit);
        Ok(AgentMemorySearchResult {
            memories: memories
                .into_iter()
                .map(memory_view)
                .collect::<Result<_, _>>()?,
            total,
        })
    }

    pub(crate) async fn read_memory_request(
        &self,
        request: &AgentWorkerMemoryRequest,
    ) -> Result<AgentMemorySearchResult, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let claimed = self
            .worker_run_for_lease_request(&AgentWorkerRunLeaseRequest {
                run_id: request.run_id,
                lease: request.lease.clone(),
            })
            .await?;
        let context = self.load_run_context(&claimed).await?;
        self.read_memory_for_run(
            &context,
            ReadAgentMemoryInput {
                query: request.query.clone(),
                limit: request.limit,
            },
        )
        .await
    }
}

#[async_trait::async_trait]
impl AgentWorkerMemoryReadApplicationPort for AgentApplication {
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
}
