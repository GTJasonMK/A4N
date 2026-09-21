//! Adapters from the generation owner to Host contracts.

use super::*;

struct OwnerAuthorizedGenerationTaskAdapter {
    context: OrchestratorContext,
    task: AuthorizedGenerationTask,
}

impl HostAuthorizedGenerationTaskPort for OwnerAuthorizedGenerationTaskAdapter {
    fn public_snapshot(&self) -> Option<&GenerationTaskSnapshot> {
        self.task.public_snapshot()
    }

    fn snapshot_expired(&self) -> bool {
        self.task.snapshot_expired()
    }

    fn covers_through_sequence(&self) -> i64 {
        self.task.covers_through_sequence()
    }

    fn subscribe(
        &self,
        after_sequence: Option<u64>,
    ) -> Result<HostGenerationSubscription, HostGenerationTaskAccessError> {
        let subscription = self
            .context
            .subscribe_generation_task(&self.task, after_sequence)
            .map_err(owner_access_error)?;
        Ok(HostGenerationSubscription::new(Box::new(
            OwnerGenerationSubscriptionAdapter {
                inner: subscription,
            },
        )))
    }
}

struct OwnerGenerationSubscriptionAdapter {
    inner: OwnerGenerationSubscription,
}

impl HostGenerationSubscriptionPort for OwnerGenerationSubscriptionAdapter {
    fn take_initial(&mut self) -> Option<HostGenerationReplay> {
        self.inner.take_initial().map(owner_replay)
    }

    fn receiver(&mut self) -> &mut tokio::sync::watch::Receiver<u64> {
        self.inner.receiver()
    }

    fn replay_after(&self, after_sequence: Option<u64>) -> HostGenerationReplay {
        owner_replay(self.inner.replay_after(after_sequence))
    }

    fn snapshot(&self) -> GenerationTaskSnapshot {
        self.inner.snapshot()
    }
}

fn owner_replay(replay: OwnerGenerationReplay) -> HostGenerationReplay {
    match replay {
        OwnerGenerationReplay::Events {
            events,
            latest_sequence,
            replay_floor,
        } => HostGenerationReplay::Events {
            events,
            latest_sequence,
            replay_floor,
        },
        OwnerGenerationReplay::SnapshotRequired {
            snapshot,
            latest_sequence,
            replay_floor,
        } => HostGenerationReplay::SnapshotRequired {
            snapshot,
            latest_sequence,
            replay_floor,
        },
    }
}

fn owner_access_error(error: GenerationTaskAccessError) -> HostGenerationTaskAccessError {
    let code = error.code();
    let retryable = !matches!(
        code,
        "generation_request_invalid"
            | "generation_task_forbidden"
            | "generation_task_unauthenticated"
            | "generation_task_not_found"
    );
    HostGenerationTaskAccessError::new(code, error.to_string(), retryable)
}

fn owner_terminal_error(error: GenerationTerminalError) -> HostGenerationTerminalError {
    let code = error.code();
    let retryable = !matches!(
        code,
        "generation_attempt_mismatch"
            | "generation_event_invalid"
            | "generation_terminal_already_set"
    );
    HostGenerationTerminalError::new(code, error.to_string(), retryable)
}

fn owner_lifecycle_error(error: GenerationLifecycleGateError) -> HostGenerationLifecycleError {
    let code = error.code();
    let retryable = !matches!(
        code,
        "generation_request_invalid" | "generation_task_not_found"
    );
    HostGenerationLifecycleError::new(code, error.to_string(), retryable)
}

fn owner_cleanup_error(error: GenerationTaskLifecycleError) -> HostGenerationLifecycleError {
    let code = error.code();
    let retryable = !matches!(
        code,
        "generation_task_forbidden"
            | "generation_attempt_mismatch"
            | "generation_checkpoint_corrupt"
    );
    HostGenerationLifecycleError::new(code, error.to_string(), retryable)
}

fn owner_terminal_outcome(
    outcome: GenerationTerminalWriteOutcome,
) -> HostGenerationTerminalWriteOutcome {
    match outcome {
        GenerationTerminalWriteOutcome::Persisted => HostGenerationTerminalWriteOutcome::Persisted,
        GenerationTerminalWriteOutcome::RebuiltFromPersistedSnapshot => {
            HostGenerationTerminalWriteOutcome::RebuiltFromPersistedSnapshot
        }
        GenerationTerminalWriteOutcome::AlreadyTerminal => {
            HostGenerationTerminalWriteOutcome::AlreadyTerminal
        }
        GenerationTerminalWriteOutcome::NoLiveSession => {
            HostGenerationTerminalWriteOutcome::NoLiveSession
        }
    }
}

fn owner_reset_result(
    result: crate::project::ProjectGeneratedDataResetApplicationResult,
) -> HostResetApplicationResult {
    match result {
        crate::project::ProjectGeneratedDataResetApplicationResult::Standard(outcome) => {
            HostResetApplicationResult::Standard(outcome)
        }
        crate::project::ProjectGeneratedDataResetApplicationResult::Freeform { result } => {
            HostResetApplicationResult::Freeform { result }
        }
    }
}

#[async_trait::async_trait]
impl HostGenerationTaskReadApplicationPort for OrchestratorContext {
    async fn authorize_generation_task(
        &self,
        user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<HostAuthorizedGenerationTask, HostGenerationTaskAccessError> {
        OrchestratorContext::authorize_generation_task(self, user_id, task_id, attempt)
            .await
            .map(|task| {
                HostAuthorizedGenerationTask::new(Box::new(OwnerAuthorizedGenerationTaskAdapter {
                    context: self.clone(),
                    task,
                }))
            })
            .map_err(owner_access_error)
    }

    async fn generation_task_summary_for_user(
        &self,
        user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<GenerationTaskSummary, HostGenerationTaskAccessError> {
        OrchestratorContext::generation_task_summary_for_user(self, user_id, task_id, attempt)
            .await
            .map_err(owner_access_error)
    }

    async fn list_active_generation_task_summaries(
        &self,
        user_id: i64,
        project_id: Option<i64>,
    ) -> Result<Vec<GenerationTaskSummary>, HostGenerationTaskAccessError> {
        OrchestratorContext::list_active_generation_task_summaries(self, user_id, project_id)
            .await
            .map_err(owner_access_error)
    }

    async fn list_generation_task_history_summaries(
        &self,
        user_id: i64,
        project_id: Option<i64>,
        limit: u32,
        cursor: Option<GenerationTaskListCursor>,
    ) -> Result<HostGenerationTaskHistorySummaries, HostGenerationTaskAccessError> {
        OrchestratorContext::list_generation_task_history_summaries(
            self, user_id, project_id, limit, cursor,
        )
        .await
        .map(|result| HostGenerationTaskHistorySummaries {
            items: result.items,
            next_cursor: result.next_cursor,
        })
        .map_err(owner_access_error)
    }

    fn subscribe_generation_task(
        &self,
        task: &HostAuthorizedGenerationTask,
        after_sequence: Option<u64>,
    ) -> Result<HostGenerationSubscription, HostGenerationTaskAccessError> {
        task.subscribe(after_sequence)
    }
}

#[async_trait::async_trait]
impl HostGenerationTaskMutationApplicationPort for OrchestratorContext {
    async fn request_generation_cancel(
        &self,
        requested_by_user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<GenerationTaskHandle, HostGenerationTerminalError> {
        OrchestratorContext::request_generation_cancel(self, requested_by_user_id, task_id, attempt)
            .await
            .map_err(owner_terminal_error)
    }
}

#[async_trait::async_trait]
impl HostGenerationLifecycleApplicationPort for OrchestratorContext {
    async fn begin_generation_project_lifecycle_gate(
        &self,
        project_id: i64,
        user_id: i64,
        operation: ProjectLifecycleOperation,
    ) -> Result<ProjectLifecycleGateClaim, HostGenerationLifecycleError> {
        OrchestratorContext::begin_generation_project_lifecycle_gate(
            self, project_id, user_id, operation,
        )
        .await
        .map_err(owner_lifecycle_error)
    }

    async fn begin_generation_user_disable_gate(
        &self,
        caller_user_id: i64,
        user_id: i64,
    ) -> Result<UserLifecycleGateClaim, HostGenerationLifecycleError> {
        OrchestratorContext::begin_generation_user_disable_gate(self, caller_user_id, user_id)
            .await
            .map_err(owner_lifecycle_error)
    }

    async fn active_generation_tasks_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<GenerationLifecycleTask>, HostGenerationLifecycleError> {
        OrchestratorContext::active_generation_tasks_for_project(self, project_id)
            .await
            .map_err(owner_lifecycle_error)
    }

    async fn active_generation_tasks_for_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<GenerationLifecycleTask>, HostGenerationLifecycleError> {
        OrchestratorContext::active_generation_tasks_for_user(self, user_id)
            .await
            .map_err(owner_lifecycle_error)
    }

    async fn active_generation_tasks(
        &self,
    ) -> Result<Vec<GenerationLifecycleTask>, HostGenerationLifecycleError> {
        OrchestratorContext::active_generation_tasks(self)
            .await
            .map_err(owner_lifecycle_error)
    }

    async fn pending_generation_lifecycle_operations(
        &self,
    ) -> Result<Vec<PendingLifecycleOperation>, HostGenerationLifecycleError> {
        OrchestratorContext::pending_generation_lifecycle_operations(self)
            .await
            .map_err(owner_lifecycle_error)
    }

    async fn request_generation_destructive_cancel(
        &self,
        task: &GenerationLifecycleTask,
    ) -> Result<HostGenerationTerminalWriteOutcome, HostGenerationTerminalError> {
        OrchestratorContext::request_generation_destructive_cancel(self, task)
            .await
            .map(owner_terminal_outcome)
            .map_err(owner_terminal_error)
    }

    async fn interrupt_generation_destructive_task(
        &self,
        task: &GenerationLifecycleTask,
        reason: InterruptionReason,
        summary: String,
    ) -> Result<HostGenerationTerminalWriteOutcome, HostGenerationTerminalError> {
        OrchestratorContext::interrupt_generation_destructive_task(self, task, reason, summary)
            .await
            .map(owner_terminal_outcome)
            .map_err(owner_terminal_error)
    }

    async fn restore_generation_project_lifecycle_gate(
        &self,
        project_id: i64,
        operation: ProjectLifecycleOperation,
    ) -> Result<(), HostGenerationLifecycleError> {
        OrchestratorContext::restore_generation_project_lifecycle_gate(self, project_id, operation)
            .await
            .map_err(owner_lifecycle_error)
    }

    async fn restore_generation_user_lifecycle_gate(
        &self,
        user_id: i64,
    ) -> Result<(), HostGenerationLifecycleError> {
        OrchestratorContext::restore_generation_user_lifecycle_gate(self, user_id)
            .await
            .map_err(owner_lifecycle_error)
    }

    async fn complete_generation_project_reset_gate(
        &self,
        project_id: i64,
    ) -> Result<(), HostGenerationLifecycleError> {
        OrchestratorContext::complete_generation_project_reset_gate(self, project_id)
            .await
            .map_err(owner_lifecycle_error)
    }

    async fn complete_generation_user_disable_gate(
        &self,
        user_id: i64,
    ) -> Result<(), HostGenerationLifecycleError> {
        OrchestratorContext::complete_generation_user_disable_gate(self, user_id)
            .await
            .map_err(owner_lifecycle_error)
    }

    async fn delete_project_cascade_validated(&self, project_id: i64) -> Result<(), AppError> {
        OrchestratorContext::delete_project_cascade_validated(self, project_id).await
    }

    async fn reset_project_generated_data_validated(
        &self,
        project_id: i64,
    ) -> Result<HostResetApplicationResult, AppError> {
        OrchestratorContext::reset_project_generated_data_validated(self, project_id)
            .await
            .map(owner_reset_result)
    }
}

#[async_trait::async_trait]
impl HostGenerationCleanupApplicationPort for OrchestratorContext {
    async fn expire_generation_snapshot_bodies(
        &self,
        limit: usize,
    ) -> Result<u64, HostGenerationLifecycleError> {
        OrchestratorContext::expire_generation_snapshot_bodies(self, limit)
            .await
            .map_err(owner_cleanup_error)
    }

    async fn purge_expired_generation_task_trees(
        &self,
        limit: usize,
    ) -> Result<u64, HostGenerationLifecycleError> {
        OrchestratorContext::purge_expired_generation_task_trees(self, limit)
            .await
            .map_err(owner_cleanup_error)
    }
}
