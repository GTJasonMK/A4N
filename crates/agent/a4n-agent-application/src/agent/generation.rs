use super::AgentApplication;
use super::*;
use a4n_agent_runtime_contract::{
    AgentGenerationCancelApplicationPort, AgentGenerationCancelSnapshot,
};
use a4n_db::agent::{AgentOwnedTask, AgentTaskExecutionInput};
use a4n_domain::agent::AgentWorkflowOrigin;
use a4n_error::AppError;

impl AgentApplication {
    pub async fn bound_model_request(
        &self,
        run_id: i64,
        request: &mut a4n_llm_provider_contract::ChatRequest,
        max_output: usize,
    ) -> Result<(), AppError> {
        let output = request.max_tokens.unwrap_or(max_output).min(max_output);
        if output == 0 {
            return Err(AppError::InvalidArgument("模型输出上限必须大于 0".into()));
        }
        request.max_tokens = Some(output);
        request.stream_include_usage = true;
        if let Some(available) = self.run_budget_snapshot(run_id).await?.remaining.max_tokens {
            let input =
                a4n_domain::agent::agent_model_token_reservation(&request.snapshot_value()?)
                    .saturating_sub(output as u64);
            if available <= input {
                return Err(AppError::InvalidArgument(
                    "Agent 共享 token 预算不足以继续模型请求".into(),
                ));
            }
            request.max_tokens =
                Some(output.min(usize::try_from(available - input).unwrap_or(usize::MAX)));
        }
        Ok(())
    }

    pub async fn record_workflow_node_execution(
        &self,
        workflow_id: i64,
        step: &str,
        status: &str,
        input: &Value,
        artifacts: Option<&Value>,
        error: Option<&str>,
    ) -> Result<(), AppError> {
        let task = AgentOwnedTask::Workflow(workflow_id);
        let Some(owner) = self.workflow_owner(workflow_id).await? else {
            return Ok(());
        };
        let execution = if status == "running" {
            self.prepare_task_execution(AgentTaskExecutionInput {
                task: task.clone(),
                execution_key: step.into(),
                tool_name: Some("canvas_node".into()),
                arguments: input.clone(),
                input_message_id: None,
                base_input_fingerprint: None,
            })
            .await?
        } else {
            self.execution_by_key(owner.id, &task.execution_key(false, step))
                .await?
        }
        .ok_or_else(|| AppError::InvalidArgument("Canvas 节点缺少 Agent 执行记录".into()))?;
        self.record_task_execution(
            &task,
            execution.id,
            (status != "running").then(|| json!({"status": status, "artifacts": artifacts})),
            None,
            error.map(str::to_string),
        )
        .await?;
        agent::append_event(
            self.executor(),
            AgentEventInput {
                thread_id: owner.thread_id,
                turn_id: Some(owner.turn_id),
                run_id: Some(owner.id),
                execution_id: Some(execution.id),
                delegation_id: None,
                message_id: None,
                actor_kind: "agent".into(),
                actor_id: Some(owner.id.to_string()),
                kind: "professional_progress".into(),
                status: status.into(),
                public_text: format!("画布节点 {step}: {status}"),
                payload_json: Some(json!({"workflow_run_id": workflow_id, "step": step})),
            },
        )
        .await
        .map_err(agent_db_error_to_app)?;
        Ok(())
    }

    pub async fn publish_generation_progress(
        &self,
        task_id: &str,
        attempt: u32,
        step: usize,
        public_text: &str,
    ) -> Result<(), AppError> {
        let Some(owner) = self.generation_owner(task_id, attempt).await? else {
            return Ok(());
        };
        agent::append_event(
            self.executor(),
            AgentEventInput {
                thread_id: owner.thread_id,
                turn_id: Some(owner.turn_id),
                run_id: Some(owner.id),
                execution_id: None,
                delegation_id: None,
                message_id: None,
                actor_kind: "agent".into(),
                actor_id: Some(owner.id.to_string()),
                kind: "professional_progress".into(),
                status: "running".into(),
                public_text: public_text.into(),
                payload_json: Some(
                    json!({"generation_task_id": task_id, "attempt": attempt, "step": step}),
                ),
            },
        )
        .await
        .map_err(agent_db_error_to_app)?;
        Ok(())
    }

    pub async fn execution_by_key(
        &self,
        run_id: i64,
        key: &str,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        agent::execution_by_key(self.executor(), run_id, key)
            .await
            .map_err(agent_db_error_to_app)
            .map(|row| row.map(super::snapshots::execution_snapshot))
    }

    pub async fn generation_owner(
        &self,
        task_id: &str,
        attempt: u32,
    ) -> Result<Option<agent::AgentRunRecord>, AppError> {
        agent::generation_owner(self.executor(), task_id, attempt)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub async fn active_owned_generation_tasks(
        &self,
        after_task_id: &str,
    ) -> Result<Vec<agent::AgentOwnedGenerationTaskRecord>, AppError> {
        agent::active_owned_generation_tasks(self.executor(), after_task_id)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub async fn generation_owner_stopped(
        &self,
        owner: &agent::AgentRunRecord,
    ) -> Result<bool, AppError> {
        agent::generation_owner_stopped(self.executor(), owner)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub async fn prepare_task_execution(
        &self,
        input: AgentTaskExecutionInput,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError> {
        agent::create_task_execution(self.executor(), input)
            .await
            .map(|execution| execution.map(super::snapshots::execution_snapshot))
            .map_err(agent_db_error_to_app)
    }

    pub async fn record_task_execution(
        &self,
        task: &AgentOwnedTask,
        execution_id: i64,
        result: Option<Value>,
        refs: Option<Value>,
        error: Option<String>,
    ) -> Result<(), AppError> {
        agent::update_task_execution(self.executor(), task, execution_id, result, refs, error)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub async fn bind_owned_workflow(
        &self,
        origin: &AgentWorkflowOrigin,
        workflow_id: i64,
    ) -> Result<(), AppError> {
        agent::bind_owned_workflow(self.executor(), origin, workflow_id)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub async fn workflow_owner(
        &self,
        workflow_id: i64,
    ) -> Result<Option<agent::AgentRunRecord>, AppError> {
        agent::workflow_owner(self.executor(), workflow_id)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub async fn active_owned_workflows(&self, after_id: i64) -> Result<Vec<i64>, AppError> {
        agent::active_owned_workflows(self.executor(), after_id)
            .await
            .map_err(agent_db_error_to_app)
    }
}

impl AgentApplication {
    pub async fn generation_cancel_candidates(
        &self,
        after_task_id: &str,
    ) -> Result<Vec<AgentGenerationCancelSnapshot>, AppError> {
        let tasks = self.active_owned_generation_tasks(after_task_id).await?;
        let mut candidates = Vec::with_capacity(tasks.len());
        for task in tasks {
            let attempt = u32::try_from(task.attempt)
                .map_err(|_| AppError::InvalidArgument("Generation attempt 无效".into()))?;
            let owner = self
                .generation_owner(&task.task_id, attempt)
                .await?
                .ok_or_else(|| AppError::InvalidArgument("Generation owner 丢失".into()))?;
            candidates.push(AgentGenerationCancelSnapshot {
                task_id: task.task_id,
                attempt,
                requested_by_user_id: task.requested_by_user_id,
                status: task.status,
                owner_stopped: self.generation_owner_stopped(&owner).await?,
            });
        }
        Ok(candidates)
    }
}

#[async_trait::async_trait]
impl AgentGenerationCancelApplicationPort for AgentApplication {
    async fn generation_cancel_candidates(
        &self,
        after_task_id: &str,
    ) -> Result<Vec<AgentGenerationCancelSnapshot>, AppError> {
        AgentApplication::generation_cancel_candidates(self, after_task_id).await
    }
}
