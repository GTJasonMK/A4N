use std::{future::Future, sync::Arc, time::Duration};

use a4n_core::error::AppError;
use a4n_db::{
    DbHandle,
    generation::{
        GenerationCorruptRecoveryState, GenerationTaskLifecycleError,
        GenerationTaskPersistenceError, PersistGenerationLifecycleInput,
        PersistedGenerationCheckpoint, PersistedGenerationTask,
        generation_interruption_batch_summary, load_generation_corrupt_recovery_state,
        load_generation_task, lock_generation_completion, persist_generation_cancel_requested,
        persist_generation_cancel_requested_batch, persist_generation_corrupt_terminal,
        persist_generation_status_changed, persist_generation_terminal,
        persist_locked_generation_completion,
    },
};
use a4n_service_protocol::{
    BatchSummary, CancelRequestedEvent, CancellationReason, CancelledEvent, CompletedEvent,
    DraftDisposition, FailedEvent, GENERATION_EVENT_SCHEMA_VERSION, GenerationError,
    GenerationErrorCode, GenerationEvent, GenerationFinishReason, GenerationResultRef,
    GenerationTaskEvent, GenerationTaskSnapshot, GenerationTaskStatus, GenerationUsage,
    InterruptedEvent, InterruptionReason, StatusChangedEvent, StreamTextHash,
};
use futures::future::BoxFuture;
use sea_orm::{DatabaseTransaction, TransactionTrait};

use super::artifacts::PreparedArtifactEvent;
use super::events::{
    GenerationDurableCommit, GenerationSessionError, GenerationSessionRegistry,
    GenerationTaskSession, apply_committed_envelope_to_snapshot, now_ms,
};

const TERMINAL_RETRY_DELAYS: [Duration; 5] = [
    Duration::from_millis(250),
    Duration::from_secs(1),
    Duration::from_secs(3),
    Duration::from_secs(10),
    Duration::from_secs(30),
];
const TERMINAL_RETRY_LIMIT: Duration = Duration::from_secs(2 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationTerminalWriteOutcome {
    Persisted,
    RebuiltFromPersistedSnapshot,
    AlreadyTerminal,
    NoLiveSession,
}

#[derive(Debug, thiserror::Error)]
pub enum GenerationTerminalError {
    #[error(transparent)]
    Session(#[from] GenerationSessionError),
    #[error(transparent)]
    Persistence(#[from] GenerationTaskLifecycleError),
    #[error("generation task attempt 不匹配: expected={expected}, actual={actual}")]
    AttemptMismatch { expected: u32, actual: u32 },
    #[error("批量 generation task 的中断必须提供精确 batch_summary")]
    BatchSummaryRequired,
    #[error("generation terminal persistence 在 2 分钟内不可用")]
    PersistenceUnavailable,
    #[error("generation 正式结果写入失败: {0}")]
    Completion(#[source] AppError),
    #[error("generation queue 取消通知失败: {0}")]
    QueueSignal(#[source] AppError),
}

impl GenerationTerminalError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Session(error) => error.code(),
            Self::Persistence(error) => error.code(),
            Self::AttemptMismatch { .. } => "generation_attempt_mismatch",
            Self::BatchSummaryRequired => "generation_event_invalid",
            Self::PersistenceUnavailable => "generation_terminal_persistence_unavailable",
            Self::Completion(_) => "generation_persist_failed",
            Self::QueueSignal(_) => "generation_persist_failed",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CompletionDescriptor {
    pub(crate) result_ref: Option<GenerationResultRef>,
    pub(crate) final_text_hashes: Vec<StreamTextHash>,
    pub(crate) final_usage: Option<GenerationUsage>,
    pub(crate) finish_reason: Option<GenerationFinishReason>,
    pub(crate) summary: Option<String>,
    pub(crate) batch_summary: Option<BatchSummary>,
}

impl From<CompletionDescriptor> for CompletedEvent {
    fn from(descriptor: CompletionDescriptor) -> Self {
        Self {
            result_ref: descriptor.result_ref,
            final_text_hashes: descriptor.final_text_hashes,
            final_usage: descriptor.final_usage,
            finish_reason: descriptor.finish_reason,
            summary: descriptor.summary,
            batch_summary: descriptor.batch_summary,
        }
    }
}

#[derive(Clone)]
pub(crate) struct GenerationTerminalCoordinator {
    database: DbHandle,
    registry: Arc<GenerationSessionRegistry>,
}

impl GenerationTerminalCoordinator {
    pub(crate) fn new(database: DbHandle, registry: Arc<GenerationSessionRegistry>) -> Self {
        Self { database, registry }
    }

    pub(crate) fn registry(&self) -> &Arc<GenerationSessionRegistry> {
        &self.registry
    }

    pub(crate) async fn request_cancel(
        &self,
        session: &Arc<GenerationTaskSession>,
        reason: CancellationReason,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        let _guard = session.checkpoint_write_guard().await;
        let candidate = session.checkpoint_candidate();
        if matches!(
            candidate.snapshot.status,
            GenerationTaskStatus::Finalizing | GenerationTaskStatus::Completed
        ) {
            return Err(GenerationTaskLifecycleError::Conflict {
                current_status: match candidate.snapshot.status {
                    GenerationTaskStatus::Finalizing => "finalizing",
                    GenerationTaskStatus::Completed => "completed",
                    _ => unreachable!(),
                }
                .to_string(),
                current_revision: candidate.checkpoint_revision,
            }
            .into());
        }
        if candidate.snapshot.status.is_terminal() {
            return Ok(GenerationTerminalWriteOutcome::AlreadyTerminal);
        }
        if candidate.snapshot.status == GenerationTaskStatus::Cancelling {
            return Ok(GenerationTerminalWriteOutcome::Persisted);
        }
        let permit = session.prepare_committed_events(vec![GenerationEvent::CancelRequested(
            CancelRequestedEvent { reason },
        )])?;
        let input = PersistGenerationLifecycleInput {
            task_id: candidate.snapshot.task_id,
            attempt: candidate.snapshot.attempt,
            expected_checkpoint_revision: candidate.checkpoint_revision,
            expected_status: candidate.snapshot.status,
            public_snapshot: permit.snapshot().clone(),
        };
        let persisted = retry_terminal_persistence(session, || {
            persist_generation_cancel_requested(self.database.connection(), input.clone())
        })
        .await?;
        commit_or_rebuild(self.database.connection(), session, permit, persisted).await
    }

    pub(crate) async fn request_cancel_tree(
        &self,
        mut requests: Vec<(Arc<GenerationTaskSession>, CancellationReason)>,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        if requests.is_empty() {
            return Ok(GenerationTerminalWriteOutcome::NoLiveSession);
        }
        requests.sort_by(|left, right| left.0.snapshot().task_id.cmp(&right.0.snapshot().task_id));
        if requests
            .windows(2)
            .any(|pair| pair[0].0.snapshot().task_id == pair[1].0.snapshot().task_id)
        {
            return Err(GenerationSessionError::InvalidSession(
                "generation cancel tree 包含重复 task".to_string(),
            )
            .into());
        }

        let mut guards = Vec::with_capacity(requests.len());
        for (session, _) in &requests {
            guards.push(session.checkpoint_write_guard().await);
        }

        let mut inputs = Vec::new();
        let mut permits = Vec::new();
        let mut already_terminal = false;
        for (index, (session, reason)) in requests.iter().enumerate() {
            let candidate = session.checkpoint_candidate();
            match candidate.snapshot.status {
                GenerationTaskStatus::Reserved | GenerationTaskStatus::Running => {
                    let permit = session.prepare_committed_events(vec![
                        GenerationEvent::CancelRequested(CancelRequestedEvent { reason: *reason }),
                    ])?;
                    inputs.push(PersistGenerationLifecycleInput {
                        task_id: candidate.snapshot.task_id,
                        attempt: candidate.snapshot.attempt,
                        expected_checkpoint_revision: candidate.checkpoint_revision,
                        expected_status: candidate.snapshot.status,
                        public_snapshot: permit.snapshot().clone(),
                    });
                    permits.push((index, permit));
                }
                GenerationTaskStatus::Cancelling => {}
                GenerationTaskStatus::Finalizing | GenerationTaskStatus::Completed => {
                    return Err(GenerationTaskLifecycleError::Conflict {
                        current_status: match candidate.snapshot.status {
                            GenerationTaskStatus::Finalizing => "finalizing",
                            GenerationTaskStatus::Completed => "completed",
                            _ => unreachable!(),
                        }
                        .to_string(),
                        current_revision: candidate.checkpoint_revision,
                    }
                    .into());
                }
                GenerationTaskStatus::Failed
                | GenerationTaskStatus::Cancelled
                | GenerationTaskStatus::Interrupted => already_terminal = true,
            }
        }
        if inputs.is_empty() {
            return Ok(if already_terminal {
                GenerationTerminalWriteOutcome::AlreadyTerminal
            } else {
                GenerationTerminalWriteOutcome::Persisted
            });
        }

        let persisted = retry_terminal_persistence(requests[permits[0].0].0.as_ref(), || {
            persist_generation_cancel_requested_batch(self.database.connection(), inputs.clone())
        })
        .await?;
        if persisted.len() != permits.len() {
            return Err(GenerationTaskLifecycleError::Corrupt(
                "cancel_requested batch 持久化结果数量不一致".to_string(),
            )
            .into());
        }

        let mut outcome = GenerationTerminalWriteOutcome::Persisted;
        let mut first_error = None;
        for ((index, permit), persisted) in permits.into_iter().zip(persisted) {
            match commit_or_rebuild(
                self.database.connection(),
                &requests[index].0,
                permit,
                persisted,
            )
            .await
            {
                Ok(GenerationTerminalWriteOutcome::RebuiltFromPersistedSnapshot) => {
                    outcome = GenerationTerminalWriteOutcome::RebuiltFromPersistedSnapshot;
                }
                Ok(_) => {}
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }
        drop(guards);
        if let Some(error) = first_error {
            return Err(error);
        }
        Ok(outcome)
    }

    async fn begin_finalizing(
        &self,
        session: &Arc<GenerationTaskSession>,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        self.persist_status_changed(session, GenerationTaskStatus::Finalizing)
            .await
    }

    pub(crate) async fn begin_running(
        &self,
        session: &Arc<GenerationTaskSession>,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        self.persist_status_changed(session, GenerationTaskStatus::Running)
            .await
    }

    pub(crate) async fn complete_with<F>(
        &self,
        session: &Arc<GenerationTaskSession>,
        write_result: F,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError>
    where
        F: for<'tx> FnOnce(
            &'tx DatabaseTransaction,
        ) -> BoxFuture<'tx, Result<CompletionDescriptor, AppError>>,
    {
        self.complete_with_artifacts(session, Vec::new(), write_result)
            .await
    }

    pub(crate) async fn complete_with_artifacts<F>(
        &self,
        session: &Arc<GenerationTaskSession>,
        artifacts: Vec<PreparedArtifactEvent>,
        write_result: F,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError>
    where
        F: for<'tx> FnOnce(
            &'tx DatabaseTransaction,
        ) -> BoxFuture<'tx, Result<CompletionDescriptor, AppError>>,
    {
        match self.begin_finalizing(session).await? {
            GenerationTerminalWriteOutcome::AlreadyTerminal => {
                return Ok(GenerationTerminalWriteOutcome::AlreadyTerminal);
            }
            GenerationTerminalWriteOutcome::Persisted
            | GenerationTerminalWriteOutcome::RebuiltFromPersistedSnapshot => {}
            GenerationTerminalWriteOutcome::NoLiveSession => {
                return Ok(GenerationTerminalWriteOutcome::NoLiveSession);
            }
        }

        let _guard = session.checkpoint_write_guard().await;
        let candidate = session.checkpoint_candidate();
        if candidate.snapshot.status.is_terminal() {
            return Ok(GenerationTerminalWriteOutcome::AlreadyTerminal);
        }
        if candidate.snapshot.status != GenerationTaskStatus::Finalizing {
            return Err(GenerationSessionError::InvalidState {
                status: candidate.snapshot.status,
                operation: "completed",
            }
            .into());
        }

        let tx = self.database.connection().begin().await.map_err(|error| {
            GenerationTerminalError::Persistence(GenerationTaskLifecycleError::Database(error))
        })?;
        let locked = lock_generation_completion(
            &tx,
            &candidate.snapshot.task_id,
            candidate.snapshot.attempt,
            candidate.checkpoint_revision,
        )
        .await?;
        let descriptor = write_result(&tx)
            .await
            .map_err(GenerationTerminalError::Completion)?;
        validate_batch_summary_presence(&candidate.snapshot, descriptor.batch_summary.as_ref())?;
        let events =
            committed_terminal_events(artifacts, GenerationEvent::Completed(descriptor.into()));
        let permit = session.prepare_committed_events(events)?;
        let input = PersistGenerationLifecycleInput {
            task_id: candidate.snapshot.task_id,
            attempt: candidate.snapshot.attempt,
            expected_checkpoint_revision: candidate.checkpoint_revision,
            expected_status: GenerationTaskStatus::Finalizing,
            public_snapshot: permit.snapshot().clone(),
        };
        let persisted = persist_locked_generation_completion(&tx, locked, input).await?;
        tx.commit().await.map_err(|error| {
            GenerationTerminalError::Persistence(GenerationTaskLifecycleError::Database(error))
        })?;
        commit_or_rebuild(self.database.connection(), session, permit, persisted).await
    }

    async fn persist_status_changed(
        &self,
        session: &Arc<GenerationTaskSession>,
        status: GenerationTaskStatus,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        let _guard = session.checkpoint_write_guard().await;
        let candidate = session.checkpoint_candidate();
        if candidate.snapshot.status.is_terminal() {
            return Ok(GenerationTerminalWriteOutcome::AlreadyTerminal);
        }
        let permit = session.prepare_committed_events(vec![GenerationEvent::StatusChanged(
            StatusChangedEvent { status },
        )])?;
        let input = PersistGenerationLifecycleInput {
            task_id: candidate.snapshot.task_id,
            attempt: candidate.snapshot.attempt,
            expected_checkpoint_revision: candidate.checkpoint_revision,
            expected_status: candidate.snapshot.status,
            public_snapshot: permit.snapshot().clone(),
        };
        let persisted = retry_terminal_persistence(session, || {
            persist_generation_status_changed(self.database.connection(), input.clone())
        })
        .await?;
        commit_or_rebuild(self.database.connection(), session, permit, persisted).await
    }

    pub(crate) async fn fail(
        &self,
        session: &Arc<GenerationTaskSession>,
        error: GenerationError,
        batch_summary: Option<BatchSummary>,
        artifacts: Vec<PreparedArtifactEvent>,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        let _guard = session.checkpoint_write_guard().await;
        let candidate = session.checkpoint_candidate();
        if candidate.snapshot.status.is_terminal() {
            return Ok(GenerationTerminalWriteOutcome::AlreadyTerminal);
        }
        let batch_summary = self
            .resolve_batch_summary(&candidate.snapshot, batch_summary)
            .await?;
        let permit = session.prepare_committed_events(committed_terminal_events(
            artifacts,
            GenerationEvent::Failed(FailedEvent {
                error,
                draft_disposition: DraftDisposition::RetainedUntilExpiry,
                batch_summary,
            }),
        ))?;
        self.persist_live_terminal(session, candidate, permit).await
    }

    pub(crate) async fn cancel(
        &self,
        session: &Arc<GenerationTaskSession>,
        summary: Option<String>,
        batch_summary: Option<BatchSummary>,
        artifacts: Vec<PreparedArtifactEvent>,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        let _guard = session.checkpoint_write_guard().await;
        let candidate = session.checkpoint_candidate();
        if candidate.snapshot.status.is_terminal() {
            return Ok(GenerationTerminalWriteOutcome::AlreadyTerminal);
        }
        let reason = candidate.snapshot.cancel_reason.ok_or({
            GenerationSessionError::InvalidState {
                status: candidate.snapshot.status,
                operation: "cancelled",
            }
        })?;
        let batch_summary = self
            .resolve_batch_summary(&candidate.snapshot, batch_summary)
            .await?;
        let permit = session.prepare_committed_events(committed_terminal_events(
            artifacts,
            GenerationEvent::Cancelled(CancelledEvent {
                reason,
                draft_disposition: DraftDisposition::RetainedUntilExpiry,
                summary,
                batch_summary,
            }),
        ))?;
        self.persist_live_terminal(session, candidate, permit).await
    }

    pub(crate) async fn interrupt(
        &self,
        session: &Arc<GenerationTaskSession>,
        reason: InterruptionReason,
        summary: Option<String>,
        mut batch_summary: Option<BatchSummary>,
        artifacts: Vec<PreparedArtifactEvent>,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        let _guard = session.checkpoint_write_guard().await;
        let candidate = session.checkpoint_candidate();
        if candidate.snapshot.status.is_terminal() {
            return Ok(GenerationTerminalWriteOutcome::AlreadyTerminal);
        }
        batch_summary = self
            .resolve_batch_summary(&candidate.snapshot, batch_summary)
            .await?;
        let permit = session.prepare_committed_events(committed_terminal_events(
            artifacts,
            GenerationEvent::Interrupted(InterruptedEvent {
                reason,
                retryable: true,
                draft_disposition: DraftDisposition::RetainedUntilExpiry,
                summary,
                batch_summary,
            }),
        ))?;
        let input = PersistGenerationLifecycleInput {
            task_id: candidate.snapshot.task_id,
            attempt: candidate.snapshot.attempt,
            expected_checkpoint_revision: candidate.checkpoint_revision,
            expected_status: candidate.snapshot.status,
            public_snapshot: permit.snapshot().clone(),
        };
        let persisted = retry_terminal_persistence(session, || {
            persist_generation_terminal(self.database.connection(), input.clone())
        })
        .await?;
        commit_or_rebuild(self.database.connection(), session, permit, persisted).await
    }

    async fn resolve_batch_summary(
        &self,
        snapshot: &GenerationTaskSnapshot,
        mut batch_summary: Option<BatchSummary>,
    ) -> Result<Option<BatchSummary>, GenerationTerminalError> {
        if snapshot.started.batch_policy.is_some() && batch_summary.is_none() {
            batch_summary = generation_interruption_batch_summary(
                self.database.connection(),
                &snapshot.task_id,
            )
            .await?;
        }
        validate_batch_summary_presence(snapshot, batch_summary.as_ref())?;
        Ok(batch_summary)
    }

    async fn persist_live_terminal(
        &self,
        session: &Arc<GenerationTaskSession>,
        candidate: super::events::SessionCheckpointCandidate,
        permit: super::events::CommittedEventPermit,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        let input = PersistGenerationLifecycleInput {
            task_id: candidate.snapshot.task_id,
            attempt: candidate.snapshot.attempt,
            expected_checkpoint_revision: candidate.checkpoint_revision,
            expected_status: candidate.snapshot.status,
            public_snapshot: permit.snapshot().clone(),
        };
        let persisted = retry_terminal_persistence(session, || {
            persist_generation_terminal(self.database.connection(), input.clone())
        })
        .await?;
        commit_or_rebuild(self.database.connection(), session, permit, persisted).await
    }

    pub(crate) async fn interrupt_persisted_task(
        &self,
        task_id: &str,
        attempt: u32,
        reason: InterruptionReason,
        summary: Option<String>,
        mut batch_summary: Option<BatchSummary>,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        let loaded = match load_generation_task(self.database.connection(), task_id).await {
            Ok(loaded) => loaded,
            Err(GenerationTaskPersistenceError::Corrupt(details)) => {
                tracing::error!(task_id, attempt, details = %details, "恢复损坏 generation checkpoint 为 failed terminal");
                let recovery =
                    load_generation_corrupt_recovery_state(self.database.connection(), task_id)
                        .await?;
                let input = corrupt_failure_input(recovery, attempt)?;
                persist_generation_corrupt_terminal(self.database.connection(), input).await?;
                let loaded = load_generation_task(self.database.connection(), task_id)
                    .await
                    .map_err(GenerationTaskLifecycleError::from)?;
                let _ = self.restore_snapshot_only_session(&loaded)?;
                return Ok(GenerationTerminalWriteOutcome::Persisted);
            }
            Err(error) => return Err(GenerationTaskLifecycleError::from(error).into()),
        };
        let actual_attempt = u32::try_from(loaded.checkpoint.attempt).map_err(|_| {
            GenerationSessionError::InvalidSession("数据库 generation attempt 为负数".to_string())
        })?;
        if actual_attempt != attempt {
            return Err(GenerationTerminalError::AttemptMismatch {
                expected: attempt,
                actual: actual_attempt,
            });
        }
        let Some(mut snapshot) = loaded.public_snapshot.clone() else {
            return Err(GenerationSessionError::InvalidSession(
                "active generation checkpoint 缺少 public snapshot".to_string(),
            )
            .into());
        };
        if snapshot.status.is_terminal() {
            let _ = self.restore_snapshot_only_session(&loaded)?;
            return Ok(GenerationTerminalWriteOutcome::AlreadyTerminal);
        }
        if snapshot.started.batch_policy.is_some() && batch_summary.is_none() {
            batch_summary = generation_interruption_batch_summary(
                self.database.connection(),
                &snapshot.task_id,
            )
            .await?;
        }
        validate_batch_summary_presence(&snapshot, batch_summary.as_ref())?;
        let sequence = u64::try_from(loaded.checkpoint.reserved_through_sequence)
            .map_err(|_| {
                GenerationSessionError::InvalidSession(
                    "数据库 generation sequence high-water 为负数".to_string(),
                )
            })?
            .checked_add(1)
            .ok_or(GenerationSessionError::InvalidSequenceLease {
                current: u64::MAX,
                next: u64::MAX,
            })?;
        let event = GenerationTaskEvent {
            schema_version: GENERATION_EVENT_SCHEMA_VERSION,
            task_id: snapshot.task_id.clone(),
            attempt: snapshot.attempt,
            sequence,
            emitted_at_ms: now_ms()?,
            event: GenerationEvent::Interrupted(InterruptedEvent {
                reason,
                retryable: true,
                draft_disposition: DraftDisposition::RetainedUntilExpiry,
                summary,
                batch_summary,
            }),
        };
        let expected_status = snapshot.status;
        // A crashed Host may have reserved a sequence block without persisting
        // every event in it. Recovery consumes that block and starts at its
        // high-water mark, so the snapshot must advance to the lease boundary
        // before applying the terminal event; otherwise replay continuity would
        // reject the legitimate recovery event as a gap.
        snapshot.covers_through_sequence = sequence.saturating_sub(1);
        apply_committed_envelope_to_snapshot(&mut snapshot, &event)?;
        let input = PersistGenerationLifecycleInput {
            task_id: snapshot.task_id.clone(),
            attempt: snapshot.attempt,
            expected_checkpoint_revision: loaded.checkpoint.checkpoint_revision,
            expected_status,
            public_snapshot: snapshot,
        };
        let persisted = persist_generation_terminal(self.database.connection(), input).await?;
        let loaded = load_generation_task(self.database.connection(), task_id)
            .await
            .map_err(GenerationTaskLifecycleError::from)?;
        debug_assert_eq!(
            loaded.checkpoint.checkpoint_revision,
            persisted.checkpoint_revision
        );
        let _ = self.restore_snapshot_only_session(&loaded)?;
        Ok(GenerationTerminalWriteOutcome::Persisted)
    }

    pub(crate) fn restore_snapshot_only_session(
        &self,
        loaded: &PersistedGenerationTask,
    ) -> Result<Option<Arc<GenerationTaskSession>>, GenerationTerminalError> {
        let Some(snapshot) = loaded.public_snapshot.clone() else {
            return Ok(None);
        };
        if let Some(session) = self.registry.get(&snapshot.task_id, snapshot.attempt)? {
            return Ok(Some(session));
        }
        let admission = self.registry.try_admit()?;
        let session = GenerationTaskSession::from_snapshot_only(
            snapshot,
            loaded.checkpoint.checkpoint_revision,
            u64::try_from(loaded.checkpoint.reserved_through_sequence).map_err(|_| {
                GenerationSessionError::InvalidSession(
                    "数据库 generation sequence high-water 为负数".to_string(),
                )
            })?,
            loaded.checkpoint.updated_at.and_utc().timestamp_millis(),
            admission,
        )?;
        Ok(Some(self.registry.insert_or_get(session)?))
    }
}

fn corrupt_failure_input(
    recovery: GenerationCorruptRecoveryState,
    expected_attempt: u32,
) -> Result<PersistGenerationLifecycleInput, GenerationTerminalError> {
    if recovery.attempt != expected_attempt {
        return Err(GenerationTerminalError::AttemptMismatch {
            expected: expected_attempt,
            actual: recovery.attempt,
        });
    }
    let sequence = recovery.reserved_through_sequence.checked_add(1).ok_or(
        GenerationSessionError::InvalidSequenceLease {
            current: recovery.reserved_through_sequence,
            next: u64::MAX,
        },
    )?;
    let emitted_at_ms = now_ms()?;
    let event = GenerationTaskEvent {
        schema_version: GENERATION_EVENT_SCHEMA_VERSION,
        task_id: recovery.task_id.clone(),
        attempt: recovery.attempt,
        sequence,
        emitted_at_ms,
        event: GenerationEvent::Failed(FailedEvent {
            error: GenerationError {
                code: GenerationErrorCode::registered("generation_checkpoint_corrupt")
                    .map_err(|error| GenerationSessionError::InvalidSession(error.to_string()))?,
                message: "generation checkpoint 损坏，Host 已终止旧 attempt".to_string(),
                retryable: false,
                stage_code: "recovery".to_string(),
                trace_id: None,
                source_error: None,
            },
            draft_disposition: DraftDisposition::RetainedUntilExpiry,
            batch_summary: recovery.batch_summary,
        }),
    };
    Ok(PersistGenerationLifecycleInput {
        task_id: recovery.task_id.clone(),
        attempt: recovery.attempt,
        expected_checkpoint_revision: recovery.expected_checkpoint_revision,
        expected_status: recovery.expected_status,
        public_snapshot: GenerationTaskSnapshot {
            schema_version: GENERATION_EVENT_SCHEMA_VERSION,
            task_id: recovery.task_id,
            attempt: recovery.attempt,
            covers_through_sequence: sequence,
            status: GenerationTaskStatus::Failed,
            started: recovery.started,
            cancel_reason: None,
            progress: None,
            streams: Vec::new(),
            artifacts: Vec::new(),
            usage: None,
            latest_checkpoint: None,
            terminal_event: Some(event),
            updated_at_ms: emitted_at_ms,
        },
    })
}

async fn commit_or_rebuild(
    db: &a4n_db::DatabaseConnection,
    session: &Arc<GenerationTaskSession>,
    permit: super::events::CommittedEventPermit,
    persisted: PersistedGenerationCheckpoint,
) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
    let commit = durable_commit(&persisted)?;
    if permit.commit(commit).is_ok() {
        return Ok(GenerationTerminalWriteOutcome::Persisted);
    }
    let loaded = load_generation_task(db, &session.snapshot().task_id)
        .await
        .map_err(GenerationTaskLifecycleError::from)?;
    let snapshot = loaded.public_snapshot.ok_or_else(|| {
        GenerationSessionError::InvalidSession(
            "已提交 generation lifecycle 缺少 public snapshot".to_string(),
        )
    })?;
    session.rebuild_from_persisted_snapshot(
        snapshot,
        GenerationDurableCommit {
            checkpoint_revision: loaded.checkpoint.checkpoint_revision,
            covers_through_sequence: u64::try_from(loaded.checkpoint.covers_through_sequence)
                .map_err(|_| {
                    GenerationSessionError::InvalidSession(
                        "数据库 generation covers 为负数".to_string(),
                    )
                })?,
            reserved_through_sequence: u64::try_from(loaded.checkpoint.reserved_through_sequence)
                .map_err(|_| {
                GenerationSessionError::InvalidSession(
                    "数据库 generation high-water 为负数".to_string(),
                )
            })?,
            persisted_at_ms: loaded.checkpoint.updated_at.and_utc().timestamp_millis(),
        },
    )?;
    Ok(GenerationTerminalWriteOutcome::RebuiltFromPersistedSnapshot)
}

fn durable_commit(
    persisted: &PersistedGenerationCheckpoint,
) -> Result<GenerationDurableCommit, GenerationTerminalError> {
    Ok(GenerationDurableCommit {
        checkpoint_revision: persisted.checkpoint_revision,
        covers_through_sequence: u64::try_from(persisted.covers_through_sequence).map_err(
            |_| {
                GenerationSessionError::InvalidSession(
                    "数据库 generation covers 为负数".to_string(),
                )
            },
        )?,
        reserved_through_sequence: u64::try_from(persisted.reserved_through_sequence).map_err(
            |_| {
                GenerationSessionError::InvalidSession(
                    "数据库 generation high-water 为负数".to_string(),
                )
            },
        )?,
        persisted_at_ms: persisted.updated_at.and_utc().timestamp_millis(),
    })
}

async fn retry_terminal_persistence<T, F, Fut>(
    session: &GenerationTaskSession,
    mut operation: F,
) -> Result<T, GenerationTerminalError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, GenerationTaskLifecycleError>>,
{
    let started = tokio::time::Instant::now();
    let mut retry_index = 0usize;
    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(error) if retryable_database_error(&error) => {
                let elapsed = started.elapsed();
                if elapsed >= TERMINAL_RETRY_LIMIT {
                    session.seal_ordinary_publisher(
                        "generation terminal persistence unavailable".to_string(),
                    );
                    return Err(GenerationTerminalError::PersistenceUnavailable);
                }
                let delay = TERMINAL_RETRY_DELAYS[retry_index.min(TERMINAL_RETRY_DELAYS.len() - 1)]
                    .min(TERMINAL_RETRY_LIMIT - elapsed);
                retry_index = retry_index.saturating_add(1);
                tokio::time::sleep(delay).await;
            }
            Err(error) => return Err(error.into()),
        }
    }
}

fn retryable_database_error(error: &GenerationTaskLifecycleError) -> bool {
    matches!(
        error,
        GenerationTaskLifecycleError::Database(_)
            | GenerationTaskLifecycleError::Persistence(
                a4n_db::generation::GenerationTaskPersistenceError::Database(_)
            )
    )
}

fn validate_batch_summary_presence(
    snapshot: &a4n_service_protocol::GenerationTaskSnapshot,
    batch_summary: Option<&BatchSummary>,
) -> Result<(), GenerationTerminalError> {
    if snapshot.started.batch_policy.is_some() == batch_summary.is_some() {
        Ok(())
    } else {
        Err(GenerationTerminalError::BatchSummaryRequired)
    }
}

fn committed_terminal_events(
    artifacts: Vec<PreparedArtifactEvent>,
    terminal: GenerationEvent,
) -> Vec<GenerationEvent> {
    artifacts
        .into_iter()
        .map(PreparedArtifactEvent::into_event)
        .map(GenerationEvent::Artifact)
        .chain(std::iter::once(terminal))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_service_protocol::{GenerationArtifactRef, GenerationDeliveryMode, StartedEvent};

    use crate::generation::artifacts::GenerationArtifactCoordinator;

    #[test]
    fn terminal_retry_contract_is_fixed() {
        assert_eq!(
            TERMINAL_RETRY_DELAYS,
            [
                Duration::from_millis(250),
                Duration::from_secs(1),
                Duration::from_secs(3),
                Duration::from_secs(10),
                Duration::from_secs(30),
            ]
        );
        assert_eq!(TERMINAL_RETRY_LIMIT, Duration::from_secs(120));
    }

    #[test]
    fn corrupt_recovery_uses_sequence_high_water_and_failed_terminal() {
        let task_id = "67263d6c-7557-4d2f-a75f-4cd5b1c1836a".to_string();
        let input = corrupt_failure_input(
            GenerationCorruptRecoveryState {
                task_id: task_id.clone(),
                attempt: 1,
                expected_checkpoint_revision: 9,
                expected_status: GenerationTaskStatus::Running,
                reserved_through_sequence: 257,
                started: StartedEvent {
                    task_kind: "text_generation".to_string(),
                    delivery_mode: GenerationDeliveryMode::Stream,
                    project_id: None,
                    root_project_id: None,
                    workflow_run_id: None,
                    generation_run_id: None,
                    target: None,
                    batch_policy: None,
                    root_task_id: task_id,
                    parent_task_id: None,
                    item_key: None,
                    retry_of_task_id: None,
                },
                batch_summary: None,
            },
            1,
        )
        .unwrap();

        assert_eq!(input.public_snapshot.covers_through_sequence, 258);
        assert_eq!(input.public_snapshot.status, GenerationTaskStatus::Failed);
        input.public_snapshot.validate().unwrap();
    }

    #[test]
    fn committed_terminal_events_keep_artifacts_before_terminal() {
        let events = committed_terminal_events(
            vec![GenerationArtifactCoordinator::available(
                GenerationArtifactRef {
                    artifact_id: "portrait:42".to_string(),
                    artifact_kind: "portrait_image".to_string(),
                    content_type: "image/png".to_string(),
                    sha256: "a".repeat(64),
                    byte_size: 4,
                    revision: "a".repeat(64),
                    temporary: false,
                    expires_at_ms: None,
                },
                None,
                None,
                None,
            )],
            GenerationEvent::Cancelled(CancelledEvent {
                reason: CancellationReason::UserRequested,
                draft_disposition: DraftDisposition::RetainedUntilExpiry,
                summary: None,
                batch_summary: None,
            }),
        );

        assert!(matches!(events[0], GenerationEvent::Artifact(_)));
        assert!(matches!(events[1], GenerationEvent::Cancelled(_)));
    }
}
