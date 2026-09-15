use a4n_db::agent::{AgentOwnedTask, AgentTaskExecutionInput};
use a4n_db_models::models::{agent::execution, ai::optimization_session};
use a4n_error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_llm_context::{LlmContextManifest, replace_request_user_content_from_manifest};
use serde_json::{Value, json};

use crate::{OrchestratorContext, optimize::AgentLoopGenerationContext};

pub(in crate::optimize) async fn prepare_model_execution(
    context: &OrchestratorContext,
    generation: &AgentLoopGenerationContext<'_>,
    call_key: &str,
    request: &mut ChatRequest,
) -> Result<Option<execution::Model>, AppError> {
    let task = generation.session.snapshot();
    let agent = context.agent();
    let Some(owner) = agent.generation_owner(&task.task_id, task.attempt).await? else {
        return Ok(None);
    };
    let key = AgentOwnedTask::Generation {
        task_id: task.task_id.clone(),
        attempt: task.attempt,
    }
    .execution_key(true, call_key);
    let base_input_fingerprint = request
        .trace_context
        .as_ref()
        .and_then(|trace| trace.input_fingerprint_sha256.clone())
        .ok_or_else(|| AppError::InvalidArgument("内容优化模型缺少输入指纹".into()))?;
    if let Some(execution) = agent.execution_by_key(owner.id, &key).await? {
        // A completed request can precede its workflow checkpoint. Reuse its
        // exact durable request/response instead of issuing a second call.
        let original = ChatRequest::from_snapshot_value(
            execution
                .arguments_json
                .clone()
                .ok_or_else(|| AppError::InvalidArgument("内层模型执行缺少原请求".into()))?,
        )?;
        if execution
            .resource_preconditions_json
            .as_ref()
            .and_then(|v| v.get("base_input_fingerprint"))
            .and_then(Value::as_str)
            != Some(base_input_fingerprint.as_str())
        {
            return Err(AppError::InvalidArgument(
                "内容优化模型恢复请求与原输入不一致".into(),
            ));
        }
        *request = original;
        return Ok(Some(execution));
    }
    let messages = agent.user_messages_for_run(owner.id).await?;
    let input_message_id = messages
        .last()
        .map(|m| m.id)
        .ok_or_else(|| AppError::InvalidArgument("内容优化 Run 缺少用户请求".into()))?;
    // The first message is the original task already rendered into the domain
    // prompt. Keep subsequent instructions as user messages, never system text.
    if messages.len() > 1 {
        let followups: Vec<_> = messages
            .iter()
            .skip(1)
            .map(|m| {
                json!({
                    "message_id": m.id, "content": m.content,
                    "delivery_mode": m.payload_json.as_ref().and_then(|v| v.get("delivery_mode")),
                })
            })
            .collect();
        let followups = format!(
            "\n后续用户指令（按接收顺序）：\n{}",
            serde_json::to_string(&followups)?
        );
        let first_user = request
            .messages
            .iter()
            .position(|message| matches!(message.role, a4n_llm::provider::MessageRole::User));
        let last_user = request
            .messages
            .iter()
            .rposition(|message| matches!(message.role, a4n_llm::provider::MessageRole::User))
            .ok_or_else(|| AppError::InvalidArgument("内容优化请求缺少 user 消息".into()))?;
        if first_user != Some(last_user) {
            request.messages[last_user].content.push_str(&followups);
        } else {
            let mut manifest: LlmContextManifest =
                serde_json::from_value(request.context_manifest_json.clone().ok_or_else(
                    || AppError::InvalidArgument("内容优化请求缺少上下文 manifest".into()),
                )?)?;
            manifest.dynamic_user_content.push_str(&followups);
            replace_request_user_content_from_manifest(request, &manifest, &[])?;
            request.context_manifest_json = Some(serde_json::to_value(manifest)?);
        }
    }
    let output = generation
        .agent_max_output_tokens
        .ok_or_else(|| AppError::InvalidArgument("Agent 内容优化缺少模型输出上限".into()))?;
    agent.bound_model_request(owner.id, request, output).await?;
    let fingerprint = crate::optimize::optimize_llm_call_input_fingerprint_json(request)?;
    let fingerprint = crate::workflow::llm_call_checkpoint_fingerprint_sha256(&fingerprint)
        .map_err(crate::workflow::workflow_checkpoint_contract_app_error)?;
    if let Some(trace) = request.trace_context.as_mut() {
        trace.input_fingerprint_sha256 = Some(fingerprint);
    }
    agent
        .prepare_task_execution(AgentTaskExecutionInput {
            task: AgentOwnedTask::Generation {
                task_id: task.task_id,
                attempt: task.attempt,
            },
            execution_key: call_key.into(),
            tool_name: None,
            arguments: request.snapshot_value()?,
            input_message_id: Some(input_message_id),
            base_input_fingerprint: Some(base_input_fingerprint),
        })
        .await
}

pub(in crate::optimize) async fn record_model_execution(
    context: &OrchestratorContext,
    generation: &AgentLoopGenerationContext<'_>,
    execution: Option<&execution::Model>,
    result: Option<Value>,
    error: Option<String>,
) -> Result<(), AppError> {
    if let Some(execution) = execution {
        let task = generation.session.snapshot();
        context
            .agent()
            .record_task_execution(
                &AgentOwnedTask::Generation {
                    task_id: task.task_id,
                    attempt: task.attempt,
                },
                execution.id,
                result,
                None,
                error,
            )
            .await?;
    }
    Ok(())
}

pub(in crate::optimize) async fn has_new_user_input(
    context: &OrchestratorContext,
    generation: &AgentLoopGenerationContext<'_>,
) -> Result<bool, AppError> {
    let task = generation.session.snapshot();
    let agent = context.agent();
    let Some(owner) = agent.generation_owner(&task.task_id, task.attempt).await? else {
        return Ok(false);
    };
    let observed = agent
        .latest_model_input_message_id(owner.id)
        .await?
        .ok_or_else(|| AppError::InvalidArgument("内容优化最终模型缺少输入消息记录".into()))?;
    Ok(agent
        .user_messages_for_run(owner.id)
        .await?
        .iter()
        .any(|m| m.id > observed))
}

impl OrchestratorContext {
    pub(in crate::optimize) async fn execute_accounted_optimization_tool(
        &self,
        generation: &AgentLoopGenerationContext<'_>,
        session: &optimization_session::Model,
        turn_id: i64,
        step: usize,
        tool: &str,
        arguments: Value,
        context: &Value,
    ) -> Result<Value, AppError> {
        let task = generation.session.snapshot();
        let execution = self
            .agent()
            .prepare_task_execution(AgentTaskExecutionInput {
                task: AgentOwnedTask::Generation {
                    task_id: task.task_id.clone(),
                    attempt: task.attempt,
                },
                execution_key: step.to_string(),
                tool_name: Some(tool.into()),
                arguments: arguments.clone(),
                input_message_id: None,
                base_input_fingerprint: None,
            })
            .await?;
        if let Some(execution) = &execution {
            if execution.status == "succeeded" {
                return execution
                    .result_json
                    .clone()
                    .ok_or_else(|| AppError::InvalidArgument("内容优化工具缺少回执".into()));
            }
            if execution.status != "prepared" {
                return Err(AppError::InvalidArgument(format!(
                    "内容优化工具上次执行未明确完成，禁止重复派发: {}",
                    execution.id
                )));
            }
            self.agent()
                .record_task_execution(
                    &AgentOwnedTask::Generation {
                        task_id: task.task_id.clone(),
                        attempt: task.attempt,
                    },
                    execution.id,
                    None,
                    None,
                    None,
                )
                .await?;
        }
        let result = async {
            let mut result = super::execute_optimization_runtime_tool(tool, arguments, context, session.id, session.project_id, session.chapter_id)?;
            if let Some(patch) = self.persist_patch_tool_result(turn_id, session.id, session.project_id, session.chapter_id, &result).await? {
                result["patch_set_id"] = json!(patch.id);
                result["resource"] = json!({"kind": "optimization_patch_set", "id": patch.id.to_string(), "revision": patch.updated_at.and_utc().to_rfc3339(), "hash": patch.base_content_hash});
            }
            Ok::<_, AppError>(result)
        }.await;
        if let Some(execution) = &execution {
            let refs = result
                .as_ref()
                .ok()
                .and_then(|value| value.get("resource"))
                .map(|r| json!([r]));
            self.agent()
                .record_task_execution(
                    &AgentOwnedTask::Generation {
                        task_id: task.task_id,
                        attempt: task.attempt,
                    },
                    execution.id,
                    result.as_ref().ok().cloned(),
                    refs,
                    result.as_ref().err().map(ToString::to_string),
                )
                .await?;
        }
        result
    }
}
