use super::*;
use a4n_api_contract::{
    generation_tasks::{GenerationTaskHandle, GenerationTaskSummary},
    optimize::{OptimizationSessionSummary, OptimizationTurnSummary},
};
use a4n_worker_contract::{AgentRunPayload, WorkerRequest};

/// A domain entry starts the existing professional executor directly. These
/// host setup/wait calls are durable, but consume no outer model requests.
pub(super) async fn process(
    state: &AgentRuntimeState,
    claimed: &AgentWorkerRun,
    context: &AgentRunContext,
    worker_request: &WorkerRequest<AgentRunPayload>,
) -> Result<(), AppError> {
    let history_application = state.worker_history_application();
    let latest_start = context
        .executions
        .iter()
        .rev()
        .find(|e| e.execution_key.starts_with("tool-call:optimization-start"));
    let mut start_id = latest_start
        .map(|e| e.execution_key.trim_start_matches("tool-call:").to_string())
        .unwrap_or_else(|| "optimization-start".into());
    let original = context
        .messages
        .iter()
        .find(|m| m.id == context.turn.input_message_id)
        .ok_or_else(|| AppError::InvalidArgument("内容优化 Turn 缺少原始用户请求".into()))?;
    let mut args = match latest_start {
        Some(execution) => execution
            .arguments_json
            .clone()
            .ok_or_else(|| AppError::InvalidArgument("优化启动记录缺少原参数".into()))?,
        None => {
            ensure_scope_allows_tool(&context.scope, "ensure_content_optimization_session")?;
            let Some(session) = call(
                state,
                claimed,
                context,
                worker_request,
                ToolCall {
                    id: "optimization-session".into(),
                    name: "ensure_content_optimization_session".into(),
                    arguments: json!({
                        "projectId": context.scope.project_id.ok_or_else(|| {
                            AppError::InvalidArgument("内容优化需要绑定项目".into())
                        })?,
                        "chapterId": context.scope.chapter_id.ok_or_else(|| {
                            AppError::InvalidArgument("内容优化需要绑定章节".into())
                        })?
                    }),
                    stream_index: None,
                    provider_metadata: None,
                },
            )
            .await?
            else {
                return Ok(());
            };
            let session: OptimizationSessionSummary = serde_json::from_value(session)?;
            json!({"sessionId": session.id, "message": original.content})
        }
    };
    let wait_key = format!(
        "tool-call:{}",
        start_id.replacen("optimization-start", "optimization-wait", 1)
    );
    let previous_finished = context.executions.iter().any(|e| {
        e.execution_key == wait_key
            && e.status == "succeeded"
            && e.result_json
                .as_ref()
                .and_then(|v| v.pointer("/handle/status"))
                .and_then(Value::as_str)
                == Some("completed")
    });
    if previous_finished {
        let observed = history_application
            .latest_model_input_message_id(context.run.id)
            .await?
            .ok_or_else(|| AppError::InvalidArgument("内容优化缺少已处理的用户消息边界".into()))?;
        if let Some(followup) = history_application
            .user_messages_for_run(context.run.id)
            .await?
            .last()
            && followup.id > observed
        {
            start_id = format!("optimization-start:{}", followup.id);
            args["message"] = json!(format!(
                "原始目标：{}\n新追加要求：{}",
                original.content, followup.content
            ));
        }
    }
    let wait_id = start_id.replacen("optimization-start", "optimization-wait", 1);
    let session_id = args
        .get("sessionId")
        .and_then(Value::as_i64)
        .ok_or_else(|| AppError::InvalidArgument("优化启动缺少 sessionId".into()))?;
    let Some(handle) = call(
        state,
        claimed,
        context,
        worker_request,
        ToolCall {
            id: start_id,
            name: "send_content_optimization_message".into(),
            arguments: args,
            stream_index: None,
            provider_metadata: None,
        },
    )
    .await?
    else {
        return Ok(());
    };
    let handle: GenerationTaskHandle = serde_json::from_value(handle)?;
    let Some(summary) = call(
        state,
        claimed,
        context,
        worker_request,
        ToolCall {
            id: wait_id,
            name: "wait_generation_task".into(),
            arguments: json!({"taskId": handle.task_id, "attempt": handle.attempt}),
            stream_index: None,
            provider_metadata: None,
        },
    )
    .await?
    else {
        return Ok(());
    };
    let summary: GenerationTaskSummary = serde_json::from_value(summary)?;
    if summary.handle.status != a4n_service_protocol::GenerationTaskStatus::Completed {
        return Err(AppError::InvalidArgument(format!(
            "内容优化任务未完成: {:?}; {}",
            summary.handle.status,
            serde_json::to_string(&summary.terminal_event)?
        )));
    }
    let result = summary
        .result_ref
        .filter(|r| r.kind == "optimization_turn")
        .ok_or_else(|| AppError::InvalidArgument("内容优化终态缺少实际 Turn 引用".into()))?;
    let turns = state
        .dispatch_agent_host_command(
            context.owner_user_id,
            "list_optimize_turns".into(),
            json!({"sessionId": session_id}),
            worker_request,
        )
        .await?;
    let turns: Vec<OptimizationTurnSummary> = serde_json::from_value(turns)?;
    let turn = turns
        .into_iter()
        .find(|t| t.id.to_string() == result.id)
        .ok_or_else(|| AppError::InvalidArgument("内容优化结果 Turn 不存在".into()))?;
    let reply = turn
        .final_reply
        .filter(|reply| !reply.trim().is_empty())
        .ok_or_else(|| AppError::InvalidArgument("内容优化结果缺少最终答复".into()))?;
    let observed = history_application
        .latest_model_input_message_id(context.run.id)
        .await?
        .ok_or_else(|| AppError::InvalidArgument("内容优化缺少已处理的用户消息边界".into()))?;
    super::complete_run(
        state.worker_completion_application(),
        claimed,
        reply,
        observed,
    )
    .await
}

async fn call(
    state: &AgentRuntimeState,
    claimed: &AgentWorkerRun,
    context: &AgentRunContext,
    worker_request: &WorkerRequest<AgentRunPayload>,
    call: ToolCall,
) -> Result<Option<Value>, AppError> {
    let execution_application = state.worker_execution_application(claimed);
    let Some(execution) = prepare_tool_call(&execution_application, claimed, &call).await? else {
        state.wait_agent_worker_run(claimed, "budget").await?;
        return Ok(None);
    };
    let declared = context.messages.iter().any(|message| {
        message.run_id == Some(context.run.id)
            && message
                .payload_json
                .as_ref()
                .and_then(|p| p.get("tool_calls"))
                .and_then(Value::as_array)
                .is_some_and(|calls| {
                    calls
                        .iter()
                        .any(|c| c.get("id").and_then(Value::as_str) == Some(call.id.as_str()))
                })
    });
    if !declared {
        state
            .worker_message_application(claimed)
            .append_assistant_tool_message(claimed, String::new(), json!([call]))
            .await?;
    }
    let (progress, _) = execute_tool_call(
        state,
        &execution_application,
        claimed,
        context,
        &RuntimeAgentToolExecutor::new(state.clone(), worker_request.clone()),
        call,
        execution.clone(),
        &mut Vec::new(),
    )
    .await?;
    if progress == RunProgress::Released {
        return Ok(None);
    }
    let updated = state
        .load_agent_run_context(worker_request)
        .await?
        .executions
        .into_iter()
        .find(|e| e.id == execution.id)
        .ok_or_else(|| AppError::InvalidArgument("内容优化执行回执不存在".into()))?;
    if updated.status != "succeeded" {
        return Err(AppError::InvalidArgument(
            updated
                .error_message
                .unwrap_or_else(|| "内容优化调用未完成".into()),
        ));
    }
    updated
        .result_json
        .map(Some)
        .ok_or_else(|| AppError::InvalidArgument("内容优化调用缺少结果".into()))
}
