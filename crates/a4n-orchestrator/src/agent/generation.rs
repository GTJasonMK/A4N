use super::*;
pub use a4n_db::agent::{AgentOwnedTask, AgentTaskExecutionInput};
use a4n_domain::agent::AgentWorkflowOrigin;

// ChatRequest's wire serializer deliberately omits internal context. Preserve
// it in the durable Execution so replay uses the same manifest and trace.
pub fn model_request_snapshot(request: &a4n_llm::provider::ChatRequest) -> Result<Value, AppError> {
    let mut value = serde_json::to_value(request)?;
    value["trace_context"] = serde_json::to_value(&request.trace_context)?;
    value["context_manifest_json"] = request.context_manifest_json.clone().unwrap_or(Value::Null);
    Ok(value)
}

pub fn restore_model_request_snapshot(
    mut value: Value,
) -> Result<a4n_llm::provider::ChatRequest, AppError> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidArgument("模型请求快照必须是对象".into()))?;
    let trace = object.remove("trace_context");
    let manifest = object
        .remove("context_manifest_json")
        .filter(|v| !v.is_null());
    let mut request: a4n_llm::provider::ChatRequest = serde_json::from_value(value)?;
    request.trace_context = trace.map(serde_json::from_value).transpose()?.flatten();
    request.context_manifest_json = manifest;
    Ok(request)
}

impl AgentApplication {
    pub(crate) async fn bound_model_request(
        &self,
        run_id: i64,
        request: &mut a4n_llm::provider::ChatRequest,
        max_output: usize,
    ) -> Result<(), AppError> {
        let output = request.max_tokens.unwrap_or(max_output).min(max_output);
        if output == 0 {
            return Err(AppError::InvalidArgument("模型输出上限必须大于 0".into()));
        }
        request.max_tokens_profile = None;
        request.max_tokens = Some(output);
        request.stream_include_usage = true;
        if let Some(available) = self.run_budget_snapshot(run_id).await?.remaining.max_tokens {
            let input =
                a4n_domain::agent::agent_model_token_reservation(&model_request_snapshot(request)?)
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

    pub(crate) async fn record_workflow_node_execution(
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
            self.connection(),
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

    pub(crate) async fn publish_generation_progress(
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
            self.connection(),
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

    pub async fn user_messages_for_run(
        &self,
        run_id: i64,
    ) -> Result<Vec<super::AgentMessageSnapshot>, AppError> {
        Ok(agent::messages_by_run(self.connection(), run_id)
            .await
            .map_err(agent_db_error_to_app)?
            .into_iter()
            .filter(|m| m.role == "user")
            .map(Into::into)
            .collect())
    }

    pub async fn latest_model_input_message_id(
        &self,
        run_id: i64,
    ) -> Result<Option<i64>, AppError> {
        let executions = agent::executions_by_runs(self.connection(), vec![run_id])
            .await
            .map_err(agent_db_error_to_app)?;
        Ok(executions
            .iter()
            .filter(|e| e.execution_kind == "model_call" && e.status == "succeeded")
            .filter_map(|e| {
                e.resource_preconditions_json
                    .as_ref()?
                    .get("input_message_id")?
                    .as_i64()
            })
            .max())
    }

    pub async fn execution_by_key(
        &self,
        run_id: i64,
        key: &str,
    ) -> Result<Option<execution::Model>, AppError> {
        execution::Entity::find()
            .filter(execution::Column::RunId.eq(run_id))
            .filter(execution::Column::ExecutionKey.eq(key))
            .one(self.connection())
            .await
            .map_err(AppError::from)
    }

    pub async fn worker_execution_by_key(
        &self,
        run_id: i64,
        key: &str,
    ) -> Result<Option<super::AgentExecutionSnapshot>, AppError> {
        self.execution_by_key(run_id, key)
            .await
            .map(|row| row.map(Into::into))
    }
    pub async fn generation_owner(
        &self,
        task_id: &str,
        attempt: u32,
    ) -> Result<Option<run::Model>, AppError> {
        agent::generation_owner(self.connection(), task_id, attempt)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub async fn active_owned_generation_tasks(
        &self,
        after_task_id: &str,
    ) -> Result<Vec<a4n_db_models::models::generation::generation_task_checkpoint::Model>, AppError>
    {
        agent::active_owned_generation_tasks(self.connection(), after_task_id)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub async fn generation_owner_stopped(&self, owner: &run::Model) -> Result<bool, AppError> {
        agent::generation_owner_stopped(self.connection(), owner)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn prepare_task_execution(
        &self,
        input: AgentTaskExecutionInput,
    ) -> Result<Option<execution::Model>, AppError> {
        agent::create_task_execution(self.connection(), input)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn record_task_execution(
        &self,
        task: &AgentOwnedTask,
        execution_id: i64,
        result: Option<Value>,
        refs: Option<Value>,
        error: Option<String>,
    ) -> Result<(), AppError> {
        agent::update_task_execution(self.connection(), task, execution_id, result, refs, error)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub async fn bind_owned_workflow(
        &self,
        origin: &AgentWorkflowOrigin,
        workflow_id: i64,
    ) -> Result<(), AppError> {
        agent::bind_owned_workflow(self.connection(), origin, workflow_id)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub async fn workflow_owner(&self, workflow_id: i64) -> Result<Option<run::Model>, AppError> {
        agent::workflow_owner(self.connection(), workflow_id)
            .await
            .map_err(agent_db_error_to_app)
    }

    pub async fn active_owned_workflows(
        &self,
        after_id: i64,
    ) -> Result<Vec<a4n_db_models::models::generation::workflow_run::Model>, AppError> {
        agent::active_owned_workflows(self.connection(), after_id)
            .await
            .map_err(agent_db_error_to_app)
    }
}
