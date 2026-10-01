use std::time::Duration;

use a4n_domain::generation::{
    GenerationLifecycleTask, PendingLifecycleOperation, ProjectLifecycleOperation,
};
use a4n_generation_host_contract::GenerationTerminalWriteOutcome;
use a4n_service_protocol::InterruptionReason;

use crate::AppState;

pub(super) const DESTRUCTIVE_DRAIN_TIMEOUT: Duration = Duration::from_secs(30);
pub(super) const DRAIN_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub(super) enum DrainError {
    Timeout(&'static str),
    Failed(String),
}

impl DrainError {
    pub(super) fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout(_))
    }
}

impl std::fmt::Display for DrainError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout(message) => formatter.write_str(message),
            Self::Failed(message) => formatter.write_str(message),
        }
    }
}

pub(super) async fn drain_project(
    state: &AppState,
    project_id: i64,
    durable_write: &mut bool,
) -> Result<(), DrainError> {
    let started = tokio::time::Instant::now();
    loop {
        let tasks = state
            .generation_lifecycle_application()
            .active_generation_tasks_for_project(project_id)
            .await
            .map_err(|error| DrainError::Failed(error.to_string()))?;
        if tasks.is_empty() {
            return Ok(());
        }
        request_cancellation(state, &tasks, durable_write).await?;
        if started.elapsed() >= DESTRUCTIVE_DRAIN_TIMEOUT {
            interrupt_drain_timeout(state, &tasks, durable_write).await?;
            let remaining = state
                .generation_lifecycle_application()
                .active_generation_tasks_for_project(project_id)
                .await
                .map_err(|error| DrainError::Failed(error.to_string()))?;
            if remaining.is_empty() {
                return Ok(());
            }
            return Err(DrainError::Timeout(
                "generation_destructive_drain_timeout: 项目生成任务仍在 finalizing",
            ));
        }
        tokio::time::sleep(DRAIN_POLL_INTERVAL).await;
    }
}

pub(super) async fn drain_user(
    state: &AppState,
    user_id: i64,
    durable_write: &mut bool,
) -> Result<(), DrainError> {
    let started = tokio::time::Instant::now();
    loop {
        let tasks = state
            .generation_lifecycle_application()
            .active_generation_tasks_for_user(user_id)
            .await
            .map_err(|error| DrainError::Failed(error.to_string()))?;
        if tasks.is_empty() {
            return Ok(());
        }
        request_cancellation(state, &tasks, durable_write).await?;
        if started.elapsed() >= DESTRUCTIVE_DRAIN_TIMEOUT {
            interrupt_drain_timeout(state, &tasks, durable_write).await?;
            let remaining = state
                .generation_lifecycle_application()
                .active_generation_tasks_for_user(user_id)
                .await
                .map_err(|error| DrainError::Failed(error.to_string()))?;
            if remaining.is_empty() {
                return Ok(());
            }
            return Err(DrainError::Timeout(
                "generation_destructive_drain_timeout: 用户生成任务仍在 finalizing",
            ));
        }
        tokio::time::sleep(DRAIN_POLL_INTERVAL).await;
    }
}

async fn request_cancellation(
    state: &AppState,
    tasks: &[GenerationLifecycleTask],
    durable_write: &mut bool,
) -> Result<(), DrainError> {
    for task in tasks
        .iter()
        .filter(|task| task.status != "finalizing" && task.status != "cancelling")
    {
        match state
            .generation_lifecycle_application()
            .request_generation_destructive_cancel(task)
            .await
            .map_err(|error| DrainError::Failed(error.to_string()))?
        {
            GenerationTerminalWriteOutcome::Persisted
            | GenerationTerminalWriteOutcome::RebuiltFromPersistedSnapshot => {
                *durable_write = true;
            }
            GenerationTerminalWriteOutcome::NoLiveSession => {
                state
                    .generation_lifecycle_application()
                    .interrupt_generation_destructive_task(
                        task,
                        InterruptionReason::SessionOwnerLost,
                        "generation session owner 丢失，破坏性操作向前收尾".to_string(),
                    )
                    .await
                    .map_err(|error| DrainError::Failed(error.to_string()))?;
                *durable_write = true;
            }
            GenerationTerminalWriteOutcome::AlreadyTerminal => {}
        }
    }
    Ok(())
}

async fn interrupt_drain_timeout(
    state: &AppState,
    tasks: &[GenerationLifecycleTask],
    durable_write: &mut bool,
) -> Result<(), DrainError> {
    for task in tasks.iter().filter(|task| task.status != "finalizing") {
        state
            .generation_lifecycle_application()
            .interrupt_generation_destructive_task(
                task,
                InterruptionReason::DestructiveDrainTimeout,
                "破坏性操作等待 30 秒后中断旧 generation attempt".to_string(),
            )
            .await
            .map_err(|error| DrainError::Failed(error.to_string()))?;
        *durable_write = true;
    }
    Ok(())
}

pub(super) async fn project_operation_pending(
    state: &AppState,
    project_id: i64,
    expected: ProjectLifecycleOperation,
) -> Result<bool, String> {
    Ok(state
        .generation_lifecycle_application()
        .pending_generation_lifecycle_operations()
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .any(|operation| {
            matches!(
                (expected, operation),
                (
                    ProjectLifecycleOperation::Delete,
                    PendingLifecycleOperation::DeleteProject { project_id: id, .. }
                ) | (
                    ProjectLifecycleOperation::Reset,
                    PendingLifecycleOperation::ResetProject { project_id: id, .. }
                ) if id == project_id
            )
        }))
}

pub(super) async fn user_disable_pending(state: &AppState, user_id: i64) -> Result<bool, String> {
    Ok(state
        .generation_lifecycle_application()
        .pending_generation_lifecycle_operations()
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .any(|operation| {
            matches!(
                operation,
                PendingLifecycleOperation::DisableUser { user_id: id } if id == user_id
            )
        }))
}

pub(super) async fn interrupt_startup_tasks(
    state: &AppState,
    tasks: Vec<GenerationLifecycleTask>,
) -> Result<(), String> {
    for task in tasks {
        state
            .generation_lifecycle_application()
            .interrupt_generation_destructive_task(
                &task,
                InterruptionReason::HostRestartRecovery,
                "Host 重启恢复时中断旧 generation attempt".to_string(),
            )
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(super) async fn restore_project_if_safe(
    state: &AppState,
    project_id: i64,
    operation: ProjectLifecycleOperation,
    restore: bool,
) {
    if restore
        && let Err(error) = state
            .generation_lifecycle_application()
            .restore_generation_project_lifecycle_gate(project_id, operation)
            .await
    {
        tracing::error!(project_id, error = %error, "恢复 project lifecycle gate 失败");
    }
}
