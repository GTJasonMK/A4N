use std::{collections::BTreeMap, future::Future, sync::Arc, time::Duration};

use a4n_core::error::AppError;
use a4n_db::generation::{
    GenerationLifecycleTask, LifecycleGateClaimState, PendingLifecycleOperation,
    ProjectLifecycleOperation,
};
use a4n_orchestrator::{
    generation::terminal::GenerationTerminalWriteOutcome,
    project::ProjectGeneratedDataResetApplicationResult,
};
use a4n_service_protocol::InterruptionReason;

use crate::AppState;

const DESTRUCTIVE_DRAIN_TIMEOUT: Duration = Duration::from_secs(30);
const DRAIN_POLL_INTERVAL: Duration = Duration::from_millis(100);
const CONTINUATION_RETRY_INTERVAL: Duration = Duration::from_secs(1);

enum DeleteProjectForwardError {
    Rejected(String),
    Retryable(String),
}

#[derive(Default)]
pub(crate) struct DestructiveLifecycleCoordinator {
    // ponytail: target 数量受 user/project 数量约束；只有压测证明常驻 key 成本可见时再做弱引用清理。
    target_locks: std::sync::Mutex<BTreeMap<String, Arc<tokio::sync::Mutex<()>>>>,
    continuations: std::sync::Mutex<BTreeMap<String, tokio::task::JoinHandle<()>>>,
}

impl DestructiveLifecycleCoordinator {
    pub(crate) async fn delete_project(
        self: &Arc<Self>,
        state: &AppState,
        user_id: i64,
        project_id: i64,
    ) -> Result<(), String> {
        let _guard = self.lock(format!("project:{project_id}")).await;
        let claim = state
            .orchestrator()
            .begin_generation_project_lifecycle_gate(
                project_id,
                user_id,
                ProjectLifecycleOperation::Delete,
            )
            .await
            .map_err(|error| error.to_string())?;
        let can_restore = claim.state == LifecycleGateClaimState::Started;
        let mut durable_drain_write = false;
        if let Err(error) = drain_project(state, project_id, &mut durable_drain_write).await {
            if !can_restore || durable_drain_write || error.is_timeout() {
                self.schedule_project_continuation(
                    state,
                    project_id,
                    ProjectLifecycleOperation::Delete,
                );
            } else {
                restore_project_if_safe(state, project_id, ProjectLifecycleOperation::Delete, true)
                    .await;
            }
            return Err(error.to_string());
        }
        match delete_project_forward(state, project_id).await {
            Ok(()) => Ok(()),
            Err(DeleteProjectForwardError::Rejected(error)) => Err(error),
            Err(DeleteProjectForwardError::Retryable(error)) => {
                self.schedule_project_continuation(
                    state,
                    project_id,
                    ProjectLifecycleOperation::Delete,
                );
                Err(error)
            }
        }
    }

    pub(crate) async fn reset_project(
        self: &Arc<Self>,
        state: &AppState,
        user_id: i64,
        project_id: i64,
    ) -> Result<ProjectGeneratedDataResetApplicationResult, String> {
        let _guard = self.lock(format!("project:{project_id}")).await;
        let claim = state
            .orchestrator()
            .begin_generation_project_lifecycle_gate(
                project_id,
                user_id,
                ProjectLifecycleOperation::Reset,
            )
            .await
            .map_err(|error| error.to_string())?;
        let can_restore = claim.state == LifecycleGateClaimState::Started;
        let mut durable_drain_write = false;
        if let Err(error) = drain_project(state, project_id, &mut durable_drain_write).await {
            if !can_restore || durable_drain_write || error.is_timeout() {
                self.schedule_project_continuation(
                    state,
                    project_id,
                    ProjectLifecycleOperation::Reset,
                );
            } else {
                restore_project_if_safe(state, project_id, ProjectLifecycleOperation::Reset, true)
                    .await;
            }
            return Err(error.to_string());
        }
        match reset_project_forward(state, project_id).await {
            Ok(result) => Ok(result),
            Err(error) => {
                self.schedule_project_continuation(
                    state,
                    project_id,
                    ProjectLifecycleOperation::Reset,
                );
                Err(error.to_string())
            }
        }
    }

    pub(crate) async fn disable_user(
        self: &Arc<Self>,
        state: &AppState,
        caller_user_id: i64,
        user_id: i64,
    ) -> Result<(), String> {
        let _guard = self.lock(format!("user:{user_id}")).await;
        let claim = state
            .orchestrator()
            .begin_generation_user_disable_gate(caller_user_id, user_id)
            .await
            .map_err(|error| error.to_string())?;
        if claim.state == LifecycleGateClaimState::Complete {
            return Ok(());
        }
        let can_restore = claim.state == LifecycleGateClaimState::Started;
        let mut durable_drain_write = false;
        if let Err(error) = drain_user(state, user_id, &mut durable_drain_write).await {
            if can_restore && !durable_drain_write && !error.is_timeout() {
                let _ = state
                    .orchestrator()
                    .restore_generation_user_lifecycle_gate(user_id)
                    .await;
            } else {
                self.schedule_user_continuation(state, user_id);
            }
            return Err(error.to_string());
        }
        if let Err(error) = state
            .orchestrator()
            .complete_generation_user_disable_gate(user_id)
            .await
        {
            self.schedule_user_continuation(state, user_id);
            return Err(error.to_string());
        }
        Ok(())
    }

    pub(crate) async fn continue_pending(&self, state: &AppState) -> Result<(), String> {
        for operation in state
            .orchestrator()
            .pending_generation_lifecycle_operations()
            .await
            .map_err(|error| error.to_string())?
        {
            match operation {
                PendingLifecycleOperation::DisableUser { user_id } => {
                    let _guard = self.lock(format!("user:{user_id}")).await;
                    interrupt_startup_tasks(
                        state,
                        state
                            .orchestrator()
                            .active_generation_tasks_for_user(user_id)
                            .await
                            .map_err(|error| error.to_string())?,
                    )
                    .await?;
                    state
                        .orchestrator()
                        .complete_generation_user_disable_gate(user_id)
                        .await
                        .map_err(|error| error.to_string())?;
                }
                PendingLifecycleOperation::DeleteProject {
                    project_id,
                    user_id: _,
                    root_project_id: _,
                } => {
                    let _guard = self.lock(format!("project:{project_id}")).await;
                    interrupt_startup_tasks(
                        state,
                        state
                            .orchestrator()
                            .active_generation_tasks_for_project(project_id)
                            .await
                            .map_err(|error| error.to_string())?,
                    )
                    .await?;
                    match delete_project_forward(state, project_id).await {
                        Ok(()) | Err(DeleteProjectForwardError::Rejected(_)) => {}
                        Err(DeleteProjectForwardError::Retryable(error)) => return Err(error),
                    }
                }
                PendingLifecycleOperation::ResetProject {
                    project_id,
                    user_id: _,
                    root_project_id: _,
                } => {
                    let _guard = self.lock(format!("project:{project_id}")).await;
                    interrupt_startup_tasks(
                        state,
                        state
                            .orchestrator()
                            .active_generation_tasks_for_project(project_id)
                            .await
                            .map_err(|error| error.to_string())?,
                    )
                    .await?;
                    reset_project_forward(state, project_id).await?;
                }
            }
        }
        interrupt_startup_tasks(
            state,
            state
                .orchestrator()
                .active_generation_tasks()
                .await
                .map_err(|error| error.to_string())?,
        )
        .await?;
        Ok(())
    }

    async fn lock(&self, key: String) -> tokio::sync::OwnedMutexGuard<()> {
        let lock = self
            .target_locks
            .lock()
            .expect("destructive lifecycle mutex poisoned")
            .entry(key)
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone();
        lock.lock_owned().await
    }

    fn schedule_project_continuation(
        self: &Arc<Self>,
        state: &AppState,
        project_id: i64,
        operation: ProjectLifecycleOperation,
    ) {
        let coordinator = Arc::clone(self);
        let state = state.clone();
        let key = format!("project:{project_id}");
        self.spawn_once(key, async move {
            coordinator
                .continue_project_forward(state, project_id, operation)
                .await;
        });
    }

    fn schedule_user_continuation(self: &Arc<Self>, state: &AppState, user_id: i64) {
        let coordinator = Arc::clone(self);
        let state = state.clone();
        let key = format!("user:{user_id}");
        self.spawn_once(key, async move {
            coordinator.continue_user_forward(state, user_id).await;
        });
    }

    fn spawn_once(&self, key: String, future: impl Future<Output = ()> + Send + 'static) {
        let future = Box::pin(future);
        let mut continuations = self
            .continuations
            .lock()
            .expect("destructive continuation mutex poisoned");
        if continuations
            .get(&key)
            .is_some_and(|handle| !handle.is_finished())
        {
            return;
        }
        continuations.insert(key, tokio::spawn(future));
    }

    async fn continue_project_forward(
        self: Arc<Self>,
        state: AppState,
        project_id: i64,
        operation: ProjectLifecycleOperation,
    ) {
        let _guard = self.lock(format!("project:{project_id}")).await;
        loop {
            let mut durable_write = true;
            if let Err(error) = drain_project(&state, project_id, &mut durable_write).await {
                tracing::warn!(project_id, ?operation, error = %error, "后台接续 generation project lifecycle drain");
                tokio::time::sleep(CONTINUATION_RETRY_INTERVAL).await;
                continue;
            }
            let result = match operation {
                ProjectLifecycleOperation::Delete => {
                    match delete_project_forward(&state, project_id).await {
                        Ok(()) => Ok(()),
                        Err(DeleteProjectForwardError::Rejected(_)) => return,
                        Err(DeleteProjectForwardError::Retryable(error)) => Err(error),
                    }
                }
                ProjectLifecycleOperation::Reset => {
                    reset_project_forward(&state, project_id).await.map(|_| ())
                }
            };
            match result {
                Ok(()) => return,
                Err(error) => {
                    match project_operation_pending(&state, project_id, operation).await {
                        Ok(false) => return,
                        Ok(true) => {
                            tracing::warn!(project_id, ?operation, error = %error, "后台接续 generation project lifecycle 失败，将重试")
                        }
                        Err(check_error) => {
                            tracing::warn!(project_id, ?operation, error = %error, check_error = %check_error, "后台接续失败且无法复核 project gate，将重试")
                        }
                    }
                }
            }
            tokio::time::sleep(CONTINUATION_RETRY_INTERVAL).await;
        }
    }

    async fn continue_user_forward(self: Arc<Self>, state: AppState, user_id: i64) {
        let _guard = self.lock(format!("user:{user_id}")).await;
        loop {
            let mut durable_write = true;
            if let Err(error) = drain_user(&state, user_id, &mut durable_write).await {
                tracing::warn!(user_id, error = %error, "后台接续 generation user lifecycle drain");
                tokio::time::sleep(CONTINUATION_RETRY_INTERVAL).await;
                continue;
            }
            match state
                .orchestrator()
                .complete_generation_user_disable_gate(user_id)
                .await
            {
                Ok(()) => return,
                Err(error) => match user_disable_pending(&state, user_id).await {
                    Ok(false) => return,
                    Ok(true) => {
                        tracing::warn!(user_id, error = %error, "后台接续 generation user disable 失败，将重试")
                    }
                    Err(check_error) => {
                        tracing::warn!(user_id, error = %error, check_error = %check_error, "后台接续失败且无法复核 user gate，将重试")
                    }
                },
            }
            tokio::time::sleep(CONTINUATION_RETRY_INTERVAL).await;
        }
    }
}

async fn delete_project_forward(
    state: &AppState,
    project_id: i64,
) -> Result<(), DeleteProjectForwardError> {
    match state
        .orchestrator()
        .delete_project_cascade_validated(project_id)
        .await
    {
        Ok(()) => Ok(()),
        Err(error @ AppError::InvalidArgument(_)) => {
            let error = error.to_string();
            match state
                .orchestrator()
                .restore_generation_project_lifecycle_gate(
                    project_id,
                    ProjectLifecycleOperation::Delete,
                )
                .await
            {
                Ok(()) => {
                    tracing::warn!(project_id, error = %error, "项目删除不可执行，已恢复 lifecycle gate");
                    Err(DeleteProjectForwardError::Rejected(error))
                }
                Err(restore_error) => Err(DeleteProjectForwardError::Retryable(format!(
                    "{error}；恢复 project lifecycle gate 失败：{restore_error}"
                ))),
            }
        }
        Err(error) => Err(DeleteProjectForwardError::Retryable(error.to_string())),
    }
}

async fn reset_project_forward(
    state: &AppState,
    project_id: i64,
) -> Result<ProjectGeneratedDataResetApplicationResult, String> {
    let result = state
        .orchestrator()
        .reset_project_generated_data_validated(project_id)
        .await
        .map_err(|error| error.to_string())?;
    state
        .orchestrator()
        .clear_project_rag(project_id)
        .await
        .map_err(|error| error.to_string())?;
    state
        .orchestrator()
        .complete_generation_project_reset_gate(project_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(result)
}

#[derive(Debug)]
enum DrainError {
    Timeout(&'static str),
    Failed(String),
}

impl DrainError {
    fn is_timeout(&self) -> bool {
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

async fn drain_project(
    state: &AppState,
    project_id: i64,
    durable_write: &mut bool,
) -> Result<(), DrainError> {
    let started = tokio::time::Instant::now();
    loop {
        let tasks = state
            .orchestrator()
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
                .orchestrator()
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

async fn drain_user(
    state: &AppState,
    user_id: i64,
    durable_write: &mut bool,
) -> Result<(), DrainError> {
    let started = tokio::time::Instant::now();
    loop {
        let tasks = state
            .orchestrator()
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
                .orchestrator()
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
            .orchestrator()
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
                    .orchestrator()
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
            .orchestrator()
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

async fn project_operation_pending(
    state: &AppState,
    project_id: i64,
    expected: ProjectLifecycleOperation,
) -> Result<bool, String> {
    Ok(state
        .orchestrator()
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

async fn user_disable_pending(state: &AppState, user_id: i64) -> Result<bool, String> {
    Ok(state
        .orchestrator()
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

async fn interrupt_startup_tasks(
    state: &AppState,
    tasks: Vec<GenerationLifecycleTask>,
) -> Result<(), String> {
    for task in tasks {
        state
            .orchestrator()
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

async fn restore_project_if_safe(
    state: &AppState,
    project_id: i64,
    operation: ProjectLifecycleOperation,
    restore: bool,
) {
    if restore
        && let Err(error) = state
            .orchestrator()
            .restore_generation_project_lifecycle_gate(project_id, operation)
            .await
    {
        tracing::error!(project_id, error = %error, "恢复 project lifecycle gate 失败");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destructive_drain_contract_is_fixed() {
        assert_eq!(DESTRUCTIVE_DRAIN_TIMEOUT, Duration::from_secs(30));
        assert_eq!(DRAIN_POLL_INTERVAL, Duration::from_millis(100));
        assert_eq!(CONTINUATION_RETRY_INTERVAL, Duration::from_secs(1));
        assert!(DrainError::Timeout("timeout").is_timeout());
        assert!(!DrainError::Failed("failed".to_string()).is_timeout());
    }
}
