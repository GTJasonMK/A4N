use super::*;
use a4n_api_contract::agent::*;
use a4n_api_contract::chapter_visual_plan::ChapterVisualPlanView;
use a4n_worker_contract::{AgentRunPayload, WorkerRequest};

pub fn respond_agent_editor(
    state: &AppState,
    user_id: i64,
    input: RespondAgentEditorInput,
) -> impl std::future::Future<Output = Result<AgentEditorReply, AppError>> + Send + '_ {
    // Allocate here, before returning to the Host dispatcher. Boxing only at
    // the caller still reserves this large future in its debug poll stack.
    Box::pin(async move {
        let admission = state.lock_agent_admission().await;
        crate::actions::package::ensure_package_execution_enabled(state, PackageId::Agent).await?;
        let (claimed, request) = state
            .agent_editor_claim_application()
            .claim_editor_request(user_id, &input)
            .await?;
        drop(admission);
        let result = resolve_claimed_editor(state, &claimed, request, input.response).await;
        if let Err(error) = &result {
            // Once claimed, an abandoned response must not leave a fake pending UI
            // request. The normal lease recovery records uncertain writes.
            if let Err(recovery) = state
                .agent_worker_recovery_application()
                .mark_recovery_required(&claimed, error.to_string())
                .await
            {
                tracing::warn!(run_id = claimed.run.id, error = %recovery, "Could not persist editor response recovery state");
            }
        }
        state.trigger_agent_worker();
        result
    })
}

async fn resolve_claimed_editor(
    state: &AppState,
    claimed: &AgentWorkerRun,
    request: AgentEditorRequest,
    response: AgentEditorResponse,
) -> Result<AgentEditorReply, AppError> {
    let execution_application = state.agent_worker_execution_application();
    let response_application = state.agent_editor_response_application();
    let context = state
        .agent_worker_context_application()
        .load_run_context(claimed)
        .await?;
    if context.editor_source.as_ref() != Some(&request.source) {
        return Err(AppError::Forbidden("编辑器请求来源与 Turn 不一致".into()));
    }
    let execution = context
        .executions
        .iter()
        .find(|execution| execution.id.to_string() == request.execution_id)
        .ok_or_else(|| AppError::NotFound("编辑器执行不存在".into()))?;
    let call = ToolCall {
        id: execution
            .execution_key
            .strip_prefix("tool-call:")
            .ok_or_else(|| AppError::InvalidArgument("编辑器执行不是工具调用".into()))?
            .into(),
        name: execution
            .tool_name
            .clone()
            .ok_or_else(|| AppError::InvalidArgument("编辑器执行缺少工具名".into()))?,
        arguments: execution
            .arguments_json
            .clone()
            .ok_or_else(|| AppError::InvalidArgument("编辑器执行缺少原始参数".into()))?,
        stream_index: None,
        provider_metadata: None,
    };
    if a4n_agent_runtime::engine::editor_request_for_call(&context, execution, &call)? != request {
        return Err(AppError::Forbidden("编辑器响应与原始工具合同不一致".into()));
    }
    let descriptor = tool_descriptor(&call.name)?;
    if should_request_approval(&context, &call.name, &descriptor.effect)?
        && !state
            .agent_worker_approval_application()
            .approved_execution(claimed.run.thread_id, execution.id)
            .await?
    {
        return Err(AppError::Forbidden("编辑器写入缺少有效审批".into()));
    }
    if execution.status == "awaiting_user" {
        execution_application
            .update_worker_execution(WorkerExecutionUpdate {
                run_id: claimed.run.id,
                lease_token: claimed.lease_token.clone(),
                execution_id: execution.id,
                status: "dispatching".into(),
                result_json: None,
                result_summary: None,
                result_refs_json: None,
                error_message: None,
            })
            .await?;
    }
    execution_application
        .update_worker_execution(WorkerExecutionUpdate {
            run_id: claimed.run.id,
            lease_token: claimed.lease_token.clone(),
            execution_id: execution.id,
            status: "running".into(),
            result_json: None,
            result_summary: None,
            result_refs_json: None,
            error_message: None,
        })
        .await?;
    let worker_id = claimed
        .run
        .worker_id
        .as_deref()
        .ok_or_else(|| AppError::InvalidState {
            current: format!("Agent run {} 缺少 worker_id", claimed.run.id),
            operation: "恢复编辑器 Agent worker request".into(),
        })?;
    let worker_request = super::worker_request_for_claimed_run(
        claimed,
        worker_id,
        state.agent_runtime_snapshot(claimed.run.id).await,
    )?;
    let outcome =
        resolve_response(state, &context, &request, &call, response, &worker_request).await;
    let (status, value, saved_draft, error_message) = match outcome {
        Ok((value, saved)) => (AgentExecutionStatus::Succeeded, value, saved, None),
        Err(error) => {
            let error = error.to_string();
            (
                AgentExecutionStatus::Failed,
                json!({"error": error}),
                None,
                Some(error),
            )
        }
    };
    execution_application
        .update_worker_execution(WorkerExecutionUpdate {
            run_id: claimed.run.id,
            lease_token: claimed.lease_token.clone(),
            execution_id: execution.id,
            status: enum_string(status),
            result_json: error_message.is_none().then(|| value.clone()),
            result_summary: error_message.is_none().then(|| "原编辑器请求已完成".into()),
            result_refs_json: if error_message.is_none() {
                Some(serde_json::to_value(tool_result_refs(&call.name, &value)?)?)
            } else {
                None
            },
            error_message: error_message.clone(),
        })
        .await?;
    append_tool_observation(
        state.agent_worker_message_application(),
        claimed,
        execution,
        &call,
        value,
        &mut Vec::new(),
    )
    .await?;
    response_application
        .requeue_after_external_response(claimed)
        .await?;
    Ok(AgentEditorReply {
        execution_id: request.execution_id,
        status,
        saved_draft,
        error_message,
    })
}

async fn resolve_response(
    state: &AppState,
    context: &AgentRunContext,
    request: &AgentEditorRequest,
    call: &ToolCall,
    response: AgentEditorResponse,
    worker_request: &WorkerRequest<AgentRunPayload>,
) -> Result<(Value, Option<ChapterVisualPlanView>), AppError> {
    let budget: AgentBudget = serde_json::from_value(context.budget.clone())?;
    validate_agent_budget(&budget)?;
    if action_budget_exhausted(
        &budget,
        json_u64(&context.usage, "tokens_used")?,
        context.run.started_at.unwrap_or(context.run.created_at),
    ) {
        return Err(AppError::InvalidArgument(
            "Agent 预算已耗尽，编辑器操作未执行".into(),
        ));
    }
    match (&request.operation, response) {
        (_, AgentEditorResponse::Error { message }) => Err(AppError::InvalidArgument(message)),
        (AgentEditorOperation::Inspect, AgentEditorResponse::State { state }) => {
            validate_state(&state)?;
            Ok((
                serde_json::to_value(AgentEditorTarget {
                    source: request.source.clone(),
                    state,
                })?,
                None,
            ))
        }
        (
            AgentEditorOperation::Read { input },
            AgentEditorResponse::Draft {
                editor_revision,
                plan,
            },
        ) => {
            check_editor_revision(&input.expected_editor_revision, &editor_revision)?;
            let projection = state
                .chapter_visual_plan_read_application()
                .project_editor(&plan, &input.view)?;
            Ok((
                serde_json::to_value(AgentEditorDraftView {
                    source: "editor_draft".into(),
                    view_instance_id: request.source.view_instance_id.clone(),
                    editor_revision,
                    projection,
                })?,
                None,
            ))
        }
        (
            AgentEditorOperation::Save { input },
            AgentEditorResponse::SaveDraft {
                state: editor,
                plan,
            },
        ) => {
            validate_state(&editor)?;
            check_editor_revision(&input.expected_editor_revision, &editor.editor_revision)?;
            if editor.has_uncommitted_dialog {
                return Err(AppError::InvalidArgument(
                    "弹窗仍有未提交输入，请先完成或关闭弹窗".into(),
                ));
            }
            crate::actions::package::ensure_command_available(
                state,
                context.owner_user_id,
                "save_chapter_visual_plan_draft",
            )
            .await?;
            let saved =
                crate::actions::chapter_visual_plan_records::save_chapter_visual_plan_draft(
                    state,
                    context.owner_user_id,
                    SaveChapterVisualPlanDraftInput {
                        project_id: request.source.project_id,
                        source_chapter_id: request.source.chapter_id,
                        authoring_mode: ChapterVisualPlanAuthoringMode::Collaborative,
                        record_id: editor.record_id,
                        expected_revision: editor.record_revision,
                        plan,
                        expected_source_hash: None,
                        editor_revision: None,
                    },
                )
                .await
                .map_err(AppError::InvalidArgument)?;
            let resource = tool_result_refs(
                "save_chapter_visual_plan_draft",
                &serde_json::to_value(&saved)?,
            )?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::InvalidState {
                current: "保存回执缺少资源引用".into(),
                operation: "保存编辑器草稿".into(),
            })?;
            Ok((
                serde_json::to_value(AgentEditorSavedView {
                    resource,
                    editor_revision: editor.editor_revision,
                })?,
                Some(saved),
            ))
        }
        (
            AgentEditorOperation::PrepareWrite {
                expected_editor_revision,
                record_id,
                record_revision,
            },
            AgentEditorResponse::ReadyToWrite { state: editor },
        ) => {
            validate_state(&editor)?;
            check_editor_revision(expected_editor_revision, &editor.editor_revision)?;
            if editor.dirty || editor.has_uncommitted_dialog {
                return Err(AppError::InvalidArgument(
                    "原编辑器仍有未保存修改，请按需调用 save_editor_draft 后重新读取版本".into(),
                ));
            }
            if editor.record_id != *record_id || editor.record_revision != *record_revision {
                return Err(AppError::InvalidArgument(
                    "原编辑器已保存版本发生变化，请重新读取后提交".into(),
                ));
            }
            let value = RuntimeAgentToolExecutor::new(state.clone(), worker_request.clone())
                .execute(context, &call.name, call.arguments.clone(), true, None)
                .await?;
            Ok((value, None))
        }
        _ => Err(AppError::InvalidArgument(
            "编辑器响应类型与请求不匹配".into(),
        )),
    }
}

fn check_editor_revision(expected: &str, current: &str) -> Result<(), AppError> {
    if expected.is_empty() || expected != current {
        return Err(AppError::InvalidArgument(
            "编辑器版本已变化，旧版本不可读取或写入；请重新 inspect_editor_target".into(),
        ));
    }
    Ok(())
}

fn validate_state(state: &AgentEditorState) -> Result<(), AppError> {
    if state.editor_revision.is_empty()
        || state.editor_revision.len() > 128
        || state.record_id.is_some() != state.record_revision.is_some()
        || state.record_id.is_some_and(|id| id <= 0)
        || state.record_revision.is_some_and(|revision| revision <= 0)
    {
        return Err(AppError::InvalidArgument(
            "编辑器状态或已保存版本无效".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_editor_versions_and_half_bound_records_are_rejected() {
        assert!(check_editor_revision("v1", "v2").is_err());
        assert!(check_editor_revision("", "").is_err());
        assert!(check_editor_revision("v1", "v1").is_ok());
        assert!(
            validate_state(&AgentEditorState {
                editor_revision: "v1".into(),
                record_id: Some(1),
                record_revision: None,
                dirty: false,
                has_uncommitted_dialog: false
            })
            .is_err()
        );
    }
}
