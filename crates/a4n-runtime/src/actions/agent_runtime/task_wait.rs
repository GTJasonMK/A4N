use super::*;
use a4n_orchestrator::agent::{
    AgentExternalTaskWaitSnapshot, AgentGenerationCancelSnapshot, AgentWorkflowCancelSnapshot,
};

pub(super) fn is_wait(name: &str) -> bool {
    matches!(name, "wait_generation_task" | "wait_workflow_run")
}

pub(super) fn is_terminal(name: &str, value: &Value) -> Result<bool, AppError> {
    match name {
        "wait_generation_task" => Ok(serde_json::from_value::<
            a4n_api_contract::generation_tasks::GenerationTaskSummary,
        >(value.clone())?
        .handle
        .status
        .is_terminal()),
        "wait_workflow_run" => {
            let detail: a4n_api_contract::workflow::WorkflowRunDetailSummary =
                serde_json::from_value(value.clone())?;
            Ok(detail.run.completed_at.is_some())
        }
        _ => Err(AppError::InvalidArgument("不是任务等待工具".into())),
    }
}

pub(super) async fn read(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    execution: &AgentExternalTaskWaitSnapshot,
) -> Result<Value, AppError> {
    let args = execution
        .arguments_json
        .clone()
        .ok_or_else(|| AppError::InvalidArgument("任务等待缺少原参数".into()))?;
    match execution.tool_name.as_deref() {
        Some("wait_generation_task") => serde_json::to_value(
            crate::actions::agent_domains::content_optimization::generation_task_summary(
                state,
                user_id,
                scope,
                parse_arguments(args, "wait_generation_task")?,
            )
            .await?,
        )
        .map_err(AppError::from),
        Some("wait_workflow_run") => serde_json::to_value(
            crate::actions::agent_domains::canvas::get_workflow_run(
                state,
                user_id,
                scope,
                parse_arguments::<HostInput<a4n_api_contract::workflow::GetWorkflowRunInput>>(
                    args,
                    "wait_workflow_run",
                )?
                .input,
            )
            .await?,
        )
        .map_err(AppError::from),
        _ => Err(AppError::InvalidArgument("任务等待工具不可用".into())),
    }
}

/// Events reduce latency; the durable scan is also used after restart and
/// covers completion between the first state read and subscription creation.
pub(super) async fn reconcile(state: &AppState) -> Result<(), AppError> {
    if let Err(error) = cancel_stopped_owned_tasks(state).await {
        tracing::error!(error = %error, "Agent 生成任务取消尚未完成，持久扫描将重试");
    }
    let wait_application = state.agent_external_task_wait_application();
    let mut cursor = 0;
    loop {
        let waits = wait_application.external_task_waits(cursor).await?;
        if waits.is_empty() {
            return Ok(());
        }
        for wait in waits {
            cursor = cursor.max(wait.run_id);
            let result = read(state, wait.owner_user_id, &wait.scope, &wait)
                .await
                .and_then(|value| {
                    let tool_name =
                        wait.tool_name
                            .as_deref()
                            .ok_or_else(|| AppError::InvalidState {
                                current: format!(
                                    "Agent waiting execution {} 缺少工具名",
                                    wait.execution_id
                                ),
                                operation: "恢复 Agent 任务等待".into(),
                            })?;
                    is_terminal(tool_name, &value)
                });
            if !matches!(result, Ok(false)) || wait.duration_exhausted {
                // The worker rechecks the authoritative result and records any
                // access/deletion error against the original call ID.
                wait_application
                    .wake_external_task_wait(wait.run_id, wait.execution_id)
                    .await?;
            }
        }
    }
}

pub(super) async fn cancel_stopped_owned_tasks(state: &AppState) -> Result<(), AppError> {
    let generation_cancel_application = state.agent_generation_cancel_application();
    let workflow_cancel_application = state.agent_workflow_cancel_application();
    let mut cursor = String::new();
    let mut first_error = None;
    loop {
        let tasks: Vec<AgentGenerationCancelSnapshot> = generation_cancel_application
            .generation_cancel_candidates(&cursor)
            .await?;
        if tasks.is_empty() {
            break;
        }
        for task in tasks {
            cursor = task.task_id.clone();
            let result = async {
                if task.owner_stopped && task.status != "cancelling" {
                    crate::actions::generation_tasks::cancel_generation_task(
                        state,
                        task.requested_by_user_id,
                        a4n_api_contract::generation_tasks::CancelGenerationTaskInput {
                            task_id: task.task_id.clone(),
                            attempt: task.attempt,
                        },
                    )
                    .await
                    .map_err(|error| {
                        AppError::InvalidArgument(format!("取消 Agent 所有的生成任务失败: {error}"))
                    })?;
                }
                Ok::<_, AppError>(())
            }
            .await;
            if let Err(error) = result {
                tracing::error!(task_id = %task.task_id, error = %error, "取消 Agent 所有的生成任务失败");
                first_error.get_or_insert(error);
            }
        }
    }
    let mut workflow_cursor = 0;
    loop {
        let workflows: Vec<AgentWorkflowCancelSnapshot> = workflow_cancel_application
            .workflow_cancel_candidates(workflow_cursor)
            .await?;
        if workflows.is_empty() {
            break;
        }
        for workflow in workflows {
            workflow_cursor = workflow.workflow_run_id;
            let result = async {
                if workflow.owner_stopped {
                    state
                        .workflow_cancellation_application()
                        .cancel(workflow.workflow_run_id, "所属 Agent 或祖先任务已停止")
                        .await?;
                }
                Ok::<_, AppError>(())
            }
            .await;
            if let Err(error) = result {
                tracing::error!(
                    workflow_run_id = workflow.workflow_run_id,
                    error = %error,
                    "取消 Agent 所有的 Workflow 失败"
                );
                first_error.get_or_insert(error);
            }
        }
    }
    first_error.map_or(Ok(()), Err)
}

pub(super) async fn release_wait(
    state: &AppState,
    claimed: &AgentWorkerRun,
    user_id: i64,
    name: &str,
    value: &Value,
) -> Result<(), AppError> {
    if name == "wait_generation_task"
        && let Err(error) = notify_on_generation_change(state, user_id, value).await
    {
        // The persisted scan is the recovery owner. A subscription failure
        // only removes fast notification; do not fail after dropping lease.
        tracing::warn!(run_id = claimed.run.id, error = %error, "Generation 通知不可用，由持久等待扫描继续对账");
    }
    state
        .agent_worker_wait_application()
        .wait_run(claimed, "external")
        .await?;
    Ok(())
}

pub(super) async fn notify_on_generation_change(
    state: &AppState,
    user_id: i64,
    value: &Value,
) -> Result<(), AppError> {
    let summary: a4n_api_contract::generation_tasks::GenerationTaskSummary =
        serde_json::from_value(value.clone())?;
    let task = crate::actions::generation_tasks::authorize_generation_task(
        state,
        user_id,
        &summary.handle.task_id,
        summary.handle.attempt,
    )
    .await
    .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
    let mut subscription =
        crate::actions::generation_tasks::subscribe_generation_task(state, &task, None)
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
    let state = state.clone();
    tokio::spawn(async move {
        if !subscription.snapshot().status.is_terminal() {
            let _ =
                tokio::time::timeout(Duration::from_secs(60), subscription.receiver().changed())
                    .await;
        }
        state.trigger_agent_worker();
    });
    Ok(())
}
