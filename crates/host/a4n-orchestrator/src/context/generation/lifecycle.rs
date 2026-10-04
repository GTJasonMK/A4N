//! Generation lifecycle gates, cancellation and cleanup coordination.

use super::read_subscribe::generation_task_handle;
use super::reservation::required_public_snapshot;
use super::*;

impl GenerationApplication {
    pub(super) fn session_for_authorized_task(
        &self,
        task: &PersistedGenerationTask,
    ) -> Result<Option<Arc<GenerationTaskSession>>, GenerationTerminalError> {
        let Some(snapshot) = task.public_snapshot.as_ref() else {
            return Ok(None);
        };
        if let Some(session) = self.session(&snapshot.task_id, snapshot.attempt)? {
            return Ok(Some(session));
        }
        self.terminal.restore_snapshot_only_session(task)
    }

    async fn begin_project_lifecycle_gate(
        &self,
        project_id: i64,
        user_id: i64,
        operation: ProjectLifecycleOperation,
    ) -> Result<ProjectLifecycleGateClaim, GenerationLifecycleGateError> {
        a4n_db::generation::begin_project_lifecycle_gate(
            self.connection(),
            project_id,
            user_id,
            operation,
        )
        .await
    }

    async fn begin_user_disable_gate(
        &self,
        caller_user_id: i64,
        user_id: i64,
    ) -> Result<UserLifecycleGateClaim, GenerationLifecycleGateError> {
        a4n_db::generation::begin_user_disable_gate(self.connection(), caller_user_id, user_id)
            .await
    }

    async fn active_generation_tasks_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError> {
        a4n_db::generation::active_generation_tasks_for_project(self.connection(), project_id).await
    }

    async fn active_generation_tasks_for_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError> {
        a4n_db::generation::active_generation_tasks_for_user(self.connection(), user_id).await
    }

    async fn active_generation_tasks(
        &self,
    ) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError> {
        a4n_db::generation::active_generation_tasks(self.connection()).await
    }

    async fn expire_generation_snapshot_bodies(
        &self,
        limit: usize,
    ) -> Result<u64, GenerationTaskLifecycleError> {
        a4n_db::generation::expire_generation_snapshot_bodies(self.connection(), limit).await
    }

    async fn purge_expired_generation_task_trees(
        &self,
        limit: usize,
    ) -> Result<u64, GenerationTaskLifecycleError> {
        a4n_db::generation::purge_expired_generation_task_trees(self.connection(), limit).await
    }

    async fn pending_lifecycle_operations(
        &self,
    ) -> Result<Vec<PendingLifecycleOperation>, GenerationLifecycleGateError> {
        a4n_db::generation::pending_lifecycle_operations(self.connection()).await
    }

    async fn request_destructive_cancel(
        &self,
        task: &GenerationLifecycleTask,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        let Some(session) = self.session(&task.task_id, task.attempt)? else {
            return Ok(GenerationTerminalWriteOutcome::NoLiveSession);
        };
        let was_reserved =
            session.snapshot().status == a4n_service_protocol::GenerationTaskStatus::Reserved;
        let outcome = self
            .terminal
            .request_cancel(
                &session,
                a4n_service_protocol::CancellationReason::Superseded,
            )
            .await?;
        self.task_queue
            .signal_generation_cancel(&task.task_id)
            .map_err(GenerationTerminalError::QueueSignal)?;
        if was_reserved {
            let outcome = self
                .terminal
                .cancel(&session, None, None, Vec::new())
                .await?;
            self.project_generation_queue_terminal(&session)?;
            return Ok(outcome);
        }
        Ok(outcome)
    }

    pub(crate) async fn request_generation_cancel(
        &self,
        requested_by_user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<GenerationTaskHandle, GenerationTerminalError> {
        let targets = a4n_db::generation::generation_cancel_targets(
            self.connection(),
            requested_by_user_id,
            task_id,
            attempt,
        )
        .await?;
        let mut live = Vec::new();
        let mut missing = Vec::new();
        for target in &targets {
            let reason = if target.task_id == task_id {
                a4n_service_protocol::CancellationReason::UserRequested
            } else {
                a4n_service_protocol::CancellationReason::ParentCancelled
            };
            match self.session(&target.task_id, target.attempt)? {
                Some(session) => live.push((target.clone(), session, reason)),
                None => missing.push((target.clone(), reason)),
            }
        }

        for (target, _) in missing {
            self.terminal
                .interrupt_persisted_task(
                    &target.task_id,
                    target.attempt,
                    a4n_service_protocol::InterruptionReason::SessionOwnerLost,
                    Some("用户取消时未找到 live generation session".to_string()),
                    None,
                )
                .await?;
        }

        let requests = live
            .iter()
            .map(|(_, session, reason)| (Arc::clone(session), *reason))
            .collect();
        self.terminal.request_cancel_tree(requests).await?;

        let mut first_queue_error = None;
        for (target, session, _) in &live {
            match self.task_queue.signal_generation_cancel(&target.task_id) {
                Ok(true) => {
                    if target.status == GenerationTaskStatus::Reserved {
                        self.terminal
                            .cancel(session, None, None, Vec::new())
                            .await?;
                        self.project_generation_queue_terminal(session)?;
                    }
                }
                Ok(false) | Err(AppError::NotFound(_)) => {
                    // A snapshot-only session can outlive the in-memory queue entry.
                    // Keep the lifecycle authoritative: reserved work can finish the
                    // cancellation immediately; running work without an executor is
                    // an interrupted attempt, not a request that remains cancelling.
                    if target.status == GenerationTaskStatus::Reserved {
                        self.terminal
                            .cancel(session, None, None, Vec::new())
                            .await?;
                    } else {
                        self.terminal
                            .interrupt(
                                session,
                                a4n_service_protocol::InterruptionReason::SessionOwnerLost,
                                Some(
                                    "用户取消时未找到 live generation execution owner".to_string(),
                                ),
                                None,
                                Vec::new(),
                            )
                            .await?;
                    }
                }
                Err(error) => {
                    first_queue_error.get_or_insert(error);
                }
            }
        }
        if let Some(error) = first_queue_error {
            return Err(GenerationTerminalError::QueueSignal(error));
        }

        let snapshot = if let Some((_, session, _)) =
            live.iter().find(|(target, _, _)| target.task_id == task_id)
        {
            session.snapshot()
        } else {
            let persisted = a4n_db::generation::load_generation_task(self.connection(), task_id)
                .await
                .map_err(GenerationTaskLifecycleError::from)?;
            required_public_snapshot(persisted).map_err(GenerationTerminalError::Completion)?
        };
        Ok(generation_task_handle(&snapshot))
    }

    fn project_generation_queue_terminal(
        &self,
        session: &GenerationTaskSession,
    ) -> Result<(), GenerationTerminalError> {
        let snapshot = session.snapshot();
        let status = match snapshot.status {
            GenerationTaskStatus::Completed => TaskStatus::Completed,
            GenerationTaskStatus::Failed => TaskStatus::Failed {
                error: "generation task failed".to_string(),
            },
            GenerationTaskStatus::Cancelled => TaskStatus::Cancelled,
            GenerationTaskStatus::Interrupted => TaskStatus::Interrupted {
                reason: Some("generation task interrupted".to_string()),
            },
            status => {
                return Err(GenerationTerminalError::QueueSignal(
                    AppError::InvalidState {
                        current: format!("{status:?}"),
                        operation: "投影 generation 终态到内存队列".to_string(),
                    },
                ));
            }
        };
        self.task_queue
            .project_generation_terminal(&snapshot.task_id, status)
            .map_err(GenerationTerminalError::QueueSignal)
    }

    async fn interrupt_destructive_task(
        &self,
        task: &GenerationLifecycleTask,
        reason: a4n_service_protocol::InterruptionReason,
        summary: String,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        if let Some(session) = self.session(&task.task_id, task.attempt)? {
            return self
                .terminal
                .interrupt(&session, reason, Some(summary), None, Vec::new())
                .await;
        }
        self.terminal
            .interrupt_persisted_task(&task.task_id, task.attempt, reason, Some(summary), None)
            .await
    }
}

impl OrchestratorContext {
    pub(crate) async fn begin_generation_project_lifecycle_gate(
        &self,
        project_id: i64,
        user_id: i64,
        operation: ProjectLifecycleOperation,
    ) -> Result<ProjectLifecycleGateClaim, GenerationLifecycleGateError> {
        self.generation
            .begin_project_lifecycle_gate(project_id, user_id, operation)
            .await
    }

    pub(crate) async fn begin_generation_user_disable_gate(
        &self,
        caller_user_id: i64,
        user_id: i64,
    ) -> Result<UserLifecycleGateClaim, GenerationLifecycleGateError> {
        self.generation
            .begin_user_disable_gate(caller_user_id, user_id)
            .await
    }

    pub(crate) async fn active_generation_tasks_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError> {
        self.generation
            .active_generation_tasks_for_project(project_id)
            .await
    }

    pub(crate) async fn active_generation_tasks_for_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError> {
        self.generation
            .active_generation_tasks_for_user(user_id)
            .await
    }

    pub(crate) async fn active_generation_tasks(
        &self,
    ) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError> {
        self.generation.active_generation_tasks().await
    }

    pub(crate) async fn expire_generation_snapshot_bodies(
        &self,
        limit: usize,
    ) -> Result<u64, GenerationTaskLifecycleError> {
        self.generation
            .expire_generation_snapshot_bodies(limit)
            .await
    }

    pub(crate) async fn purge_expired_generation_task_trees(
        &self,
        limit: usize,
    ) -> Result<u64, GenerationTaskLifecycleError> {
        self.generation
            .purge_expired_generation_task_trees(limit)
            .await
    }

    pub(crate) async fn pending_generation_lifecycle_operations(
        &self,
    ) -> Result<Vec<PendingLifecycleOperation>, GenerationLifecycleGateError> {
        self.generation.pending_lifecycle_operations().await
    }

    pub(crate) async fn request_generation_destructive_cancel(
        &self,
        task: &GenerationLifecycleTask,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        self.generation.request_destructive_cancel(task).await
    }

    pub(crate) async fn request_generation_cancel(
        &self,
        requested_by_user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<GenerationTaskHandle, GenerationTerminalError> {
        self.generation
            .request_generation_cancel(requested_by_user_id, task_id, attempt)
            .await
    }

    pub(crate) async fn interrupt_generation_destructive_task(
        &self,
        task: &GenerationLifecycleTask,
        reason: a4n_service_protocol::InterruptionReason,
        summary: String,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        self.generation
            .interrupt_destructive_task(task, reason, summary)
            .await
    }

    pub(crate) async fn restore_generation_project_lifecycle_gate(
        &self,
        project_id: i64,
        operation: ProjectLifecycleOperation,
    ) -> Result<(), GenerationLifecycleGateError> {
        a4n_db::generation::restore_project_lifecycle_gate(
            self.database.executor(),
            project_id,
            operation,
        )
        .await
    }

    pub(crate) async fn restore_generation_user_lifecycle_gate(
        &self,
        user_id: i64,
    ) -> Result<(), GenerationLifecycleGateError> {
        a4n_db::generation::restore_user_lifecycle_gate(self.database.executor(), user_id).await
    }

    pub(crate) async fn complete_generation_project_reset_gate(
        &self,
        project_id: i64,
    ) -> Result<(), GenerationLifecycleGateError> {
        a4n_db::generation::complete_project_reset_gate(self.database.executor(), project_id).await
    }

    pub(crate) async fn complete_generation_user_disable_gate(
        &self,
        user_id: i64,
    ) -> Result<(), GenerationLifecycleGateError> {
        a4n_db::generation::complete_user_disable_gate(self.database.executor(), user_id).await
    }

    pub(crate) async fn record_completed_generation_run(
        &self,
        input: RecordCompletedGenerationRunInput,
    ) -> Result<(), AppError> {
        self.generation.record_completed_generation_run(input).await
    }

    pub(crate) async fn record_completed_generation_run_with_checkpoint(
        &self,
        input: RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<i64, AppError> {
        self.generation
            .record_completed_generation_run_with_checkpoint(input)
            .await
    }

    pub(crate) async fn attach_generation_runs_to_workflow_for_target(
        &self,
        input: AttachGenerationRunsToWorkflowInput,
    ) -> Result<usize, AppError> {
        self.generation
            .attach_generation_runs_to_workflow_for_target(input)
            .await
    }

    pub(crate) async fn generation_run_exists_for_target_output(
        &self,
        target_table: &str,
        target_id: i64,
        step_key: &str,
        response_output: &str,
    ) -> Result<bool, AppError> {
        self.generation
            .generation_run_exists_for_target_output(
                target_table,
                target_id,
                step_key,
                response_output,
            )
            .await
    }
}
