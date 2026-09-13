use super::*;
use a4n_agent_application::agent::{AgentGenerationCancelSnapshot, AgentWorkflowCancelSnapshot};
use a4n_worker_contract::{
    AgentWorkerExternalTaskWaitScanRequest, AgentWorkerExternalTaskWaitSnapshotWire,
    AgentWorkerExternalTaskWaitWakeRequest,
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
    state: &AgentRuntimeState,
    _user_id: i64,
    _scope: &AgentScope,
    execution: &AgentWorkerExternalTaskWaitSnapshotWire,
) -> Result<Value, AppError> {
    let snapshot = state.agent_runtime_snapshot(execution.run_id).await;
    state
        .host()
        .read_external_task_wait(execution, snapshot.as_ref())
        .await
}

/// Events reduce latency; the durable scan is also used after restart and
/// covers completion between the first state read and subscription creation.
pub(super) async fn reconcile(state: &AgentRuntimeState) -> Result<(), AppError> {
    if let Err(error) = cancel_stopped_owned_tasks(state).await {
        tracing::error!(error = %error, "Agent 生成任务取消尚未完成，持久扫描将重试");
    }
    let mut cursor = 0;
    loop {
        let waits = state
            .host()
            .external_task_waits(&AgentWorkerExternalTaskWaitScanRequest {
                after_run_id: cursor,
            })
            .await?;
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
                state
                    .host()
                    .wake_external_task_wait(&AgentWorkerExternalTaskWaitWakeRequest {
                        run_id: wait.run_id,
                        execution_id: wait.execution_id,
                    })
                    .await?;
            }
        }
    }
}

pub(super) async fn cancel_stopped_owned_tasks(state: &AgentRuntimeState) -> Result<(), AppError> {
    let generation_cancel_application = state.generation_cancel_application();
    let workflow_cancel_application = state.workflow_cancel_application();
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
                    state
                        .host()
                        .cancel_generation_task(
                            task.requested_by_user_id,
                            task.task_id.clone(),
                            task.attempt,
                        )
                        .await
                        .map_err(|error| {
                            AppError::InvalidArgument(format!(
                                "取消 Agent 所有的生成任务失败: {error}"
                            ))
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
                        .host()
                        .cancel_workflow(workflow.workflow_run_id, "所属 Agent 或祖先任务已停止")
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
    state: &AgentRuntimeState,
    claimed: &AgentWorkerRun,
) -> Result<(), AppError> {
    state.wait_agent_worker_run(claimed, "external").await?;
    Ok(())
}
