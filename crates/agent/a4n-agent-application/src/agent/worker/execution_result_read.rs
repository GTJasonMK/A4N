use super::super::{AgentApplication, AgentRunContext};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerExecutionResultReadApplicationPort;
use a4n_api_contract::agent::{AgentExecutionResultPage, ReadAgentExecutionResultInput};
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerExecutionResultRequest;

impl AgentApplication {
    pub(crate) async fn read_execution_result_for_run(
        &self,
        context: &AgentRunContext,
        input: ReadAgentExecutionResultInput,
    ) -> Result<AgentExecutionResultPage, AppError> {
        let limit = bounded_limit(input.limit, 2_000, 2_000, "limit")? as usize;
        let execution_id = parse_id(&input.execution_id, "execution_id")?;
        let execution = agent::execution_by_id(self.executor(), execution_id)
            .await
            .map_err(agent_db_error_to_app)?
            .filter(|execution| {
                matches!(
                    execution.execution_kind.as_str(),
                    "tool_call" | "delegation"
                )
            })
            .ok_or_else(|| AppError::Forbidden("不能读取此工具结果".into()))?;
        let source_run = agent::run_by_id(self.executor(), execution.run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .filter(|run| run.thread_id == context.thread.id)
            .ok_or_else(|| AppError::Forbidden("不能读取其他 Thread 的工具结果".into()))?;
        self.owned_thread(context.owner_user_id, source_run.thread_id)
            .await?;
        let value = execution
            .result_json
            .as_ref()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("工具结果尚不可用，状态为 {}", execution.status),
                operation: "读取 Agent 工具结果".into(),
            })?;
        let selected = value.pointer(&input.pointer).ok_or_else(|| {
            AppError::InvalidArgument(format!("结果中不存在 JSON pointer: {}", input.pointer))
        })?;
        let text = selected
            .as_str()
            .map(str::to_string)
            .unwrap_or(serde_json::to_string(selected)?);
        let total_chars = text.chars().count();
        let offset = input.offset as usize;
        if offset > total_chars {
            return Err(AppError::InvalidArgument("offset 超出结果长度".into()));
        }
        let page = text.chars().skip(offset).take(limit).collect::<String>();
        let end = offset + page.chars().count();
        Ok(AgentExecutionResultPage {
            execution_id: input.execution_id,
            pointer: input.pointer,
            text: page,
            offset: input.offset,
            total_chars,
            next_offset: (end < total_chars)
                .then(|| u32::try_from(end))
                .transpose()
                .map_err(|_| AppError::InvalidArgument("结果长度超出读取范围".into()))?,
            complete: end == total_chars,
        })
    }

    pub(crate) async fn read_execution_result_request(
        &self,
        request: &AgentWorkerExecutionResultRequest,
    ) -> Result<AgentExecutionResultPage, AppError> {
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
        self.read_execution_result_for_run(
            &context,
            ReadAgentExecutionResultInput {
                execution_id: request.execution_id.clone(),
                pointer: request.pointer.clone(),
                offset: request.offset,
                limit: request.limit,
            },
        )
        .await
    }
}

#[async_trait::async_trait]
impl AgentWorkerExecutionResultReadApplicationPort for AgentApplication {
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
