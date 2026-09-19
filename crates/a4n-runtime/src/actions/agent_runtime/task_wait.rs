use super::*;
use a4n_agent_runtime_contract::{
    AgentExternalTaskWaitSnapshot, AgentGenerationCancelSnapshot, AgentWorkflowCancelSnapshot,
};

pub(crate) async fn read_with_snapshot(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    execution: &AgentExternalTaskWaitSnapshot,
    runtime_snapshot: Option<&a4n_api_contract::runtime_config::RuntimeConfigSnapshot>,
) -> Result<Value, AppError> {
    let args = execution
        .arguments_json
        .clone()
        .ok_or_else(|| AppError::InvalidArgument("任务等待缺少原参数".into()))?;
    crate::runtime_config::request_scope(user_id, runtime_snapshot.cloned(), async move {
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
    })
    .await
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
