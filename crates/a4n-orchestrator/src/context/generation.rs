//! 生成运行账本领域私有 owner。
//!
//! 持有 `DbHandle`，封装 generation_run / llm_call_trace 的落库与按目标完成/失败。

use a4n_api_contract::{
    blueprint::{StartBlueprintGenerationInput, StartBlueprintRefinementInput},
    chapter::StartChapterGenerationInput,
    generation_tasks::{
        GenerationSnapshotState, GenerationStartRequestMeta, GenerationTaskHandle,
        GenerationTaskSummary,
    },
    image_generation::{ImageGenerationOperation, StartImageGenerationInput},
    inspiration::StartInspirationGenerationInput,
    optimize::{
        StartOptimizeAnalysisGenerationInput, StartOptimizeChatGenerationInput,
        StartOptimizePatchGenerationInput,
    },
    outline::{OutlineGenerationOperation, StartOutlineGenerationInput},
    portrait::{PortraitGenerationOperation, StartPortraitGenerationInput},
    video::{StartVideoGenerationInput, VideoGenerationOperation, VideoPromptGenerationOptions},
};
use a4n_db::DbHandle;
use a4n_db::generation::{
    AttachGenerationRunsToWorkflowInput, GenerationLifecycleGateError, GenerationPublicSnapshot,
    GenerationTaskCheckpointPage, GenerationTaskLifecycleError, GenerationTaskPersistenceError,
    PersistedGenerationTask, RecordCompletedGenerationRunInput,
    RecordCompletedGenerationRunWithCheckpointInput, ReserveGenerationTaskInput,
};
use a4n_db_models::models::generation::{generation_run, llm_call_trace};
use a4n_domain::generation::{
    GenerationLifecycleTask, GenerationTaskListCursor, PendingLifecycleOperation,
    ProjectLifecycleGateClaim, ProjectLifecycleOperation, UserLifecycleGateClaim,
};
use a4n_domain::llm::RecordLlmCallTraceInput;
use a4n_domain::project::{
    BLUEPRINT_GENERATION_KIND, BLUEPRINT_REFINEMENT_TASK_KIND,
    blueprint_generation_input_fingerprint as canonical_blueprint_generation_input_fingerprint,
    blueprint_refinement_input_fingerprint as canonical_blueprint_refinement_input_fingerprint,
};
use a4n_domain::video_timeline::{TimelineError, VideoTimelineGap};
use a4n_error::AppError;
use a4n_queue::priority::{TaskKind, TaskPermit, TaskPriority, TaskStatus};
use a4n_service_protocol::{
    BatchCommitPolicy, BatchSummary, GENERATION_EVENT_SCHEMA_VERSION, GenerationDeliveryMode,
    GenerationError, GenerationEvent, GenerationFinishReason, GenerationResultRef,
    GenerationTargetRef, GenerationTaskEvent, GenerationTaskSnapshot, GenerationTaskStatus,
    InterruptionReason, StreamTextHash, TextStreamSnapshot,
};
use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use serde_json::Value;
use std::sync::Arc;

use super::OrchestratorContext;
use crate::generation::{
    artifacts::PreparedArtifactEvent,
    checkpoint::{DatabaseGenerationSequenceLeaseReplenisher, GenerationCheckpointWriter},
    events::{
        GenerationSessionConfig, GenerationSessionError, GenerationSessionRegistry,
        GenerationSubscription, GenerationTaskSession, SessionGenerationEventPublisher,
    },
    generation_error, generation_failure_is_cancelled_or_interrupted,
    terminal::{
        CompletionDescriptor, GenerationTerminalCoordinator, GenerationTerminalError,
        GenerationTerminalWriteOutcome,
    },
};
use crate::task_queue::PersistentTaskQueue;
use crate::video::validate_reference_media_start_input;
pub(crate) use a4n_api_contract::generation_tasks::GenerationTaskStart;

/// Runtime-facing generation boundary.
///
/// Generation task access, cancellation and destructive lifecycle coordination
/// stay owned by this application. Runtime only supplies host routing and the
/// external cleanup port needed by project reset.
#[async_trait::async_trait]
pub trait GenerationApplicationPort: Send + Sync {
    async fn request_generation_cancel(
        &self,
        requested_by_user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<GenerationTaskHandle, GenerationTerminalError>;

    async fn authorize_generation_task(
        &self,
        user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<AuthorizedGenerationTask, GenerationTaskAccessError>;

    async fn generation_task_summary_for_user(
        &self,
        user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<GenerationTaskSummary, GenerationTaskAccessError>;

    async fn list_active_generation_task_summaries(
        &self,
        user_id: i64,
        project_id: Option<i64>,
    ) -> Result<Vec<GenerationTaskSummary>, GenerationTaskAccessError>;

    async fn list_generation_task_history_summaries(
        &self,
        user_id: i64,
        project_id: Option<i64>,
        limit: u32,
        cursor: Option<GenerationTaskListCursor>,
    ) -> Result<GenerationTaskHistorySummaries, GenerationTaskAccessError>;

    fn subscribe_generation_task(
        &self,
        task: &AuthorizedGenerationTask,
        after_sequence: Option<u64>,
    ) -> Result<GenerationSubscription, GenerationTaskAccessError>;

    async fn begin_generation_project_lifecycle_gate(
        &self,
        project_id: i64,
        user_id: i64,
        operation: ProjectLifecycleOperation,
    ) -> Result<ProjectLifecycleGateClaim, GenerationLifecycleGateError>;

    async fn begin_generation_user_disable_gate(
        &self,
        caller_user_id: i64,
        user_id: i64,
    ) -> Result<UserLifecycleGateClaim, GenerationLifecycleGateError>;

    async fn active_generation_tasks_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError>;

    async fn active_generation_tasks_for_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError>;

    async fn active_generation_tasks(
        &self,
    ) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError>;

    async fn expire_generation_snapshot_bodies(
        &self,
        limit: usize,
    ) -> Result<u64, GenerationTaskLifecycleError>;

    async fn purge_expired_generation_task_trees(
        &self,
        limit: usize,
    ) -> Result<u64, GenerationTaskLifecycleError>;

    async fn pending_generation_lifecycle_operations(
        &self,
    ) -> Result<Vec<PendingLifecycleOperation>, GenerationLifecycleGateError>;

    async fn request_generation_destructive_cancel(
        &self,
        task: &GenerationLifecycleTask,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError>;

    async fn interrupt_generation_destructive_task(
        &self,
        task: &GenerationLifecycleTask,
        reason: InterruptionReason,
        summary: String,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError>;

    async fn restore_generation_project_lifecycle_gate(
        &self,
        project_id: i64,
        operation: ProjectLifecycleOperation,
    ) -> Result<(), GenerationLifecycleGateError>;

    async fn restore_generation_user_lifecycle_gate(
        &self,
        user_id: i64,
    ) -> Result<(), GenerationLifecycleGateError>;

    async fn complete_generation_project_reset_gate(
        &self,
        project_id: i64,
    ) -> Result<(), GenerationLifecycleGateError>;

    async fn complete_generation_user_disable_gate(
        &self,
        user_id: i64,
    ) -> Result<(), GenerationLifecycleGateError>;

    async fn delete_project_cascade_validated(&self, project_id: i64) -> Result<(), AppError>;

    async fn reset_project_generated_data_validated(
        &self,
        project_id: i64,
    ) -> Result<crate::project::ProjectGeneratedDataResetApplicationResult, AppError>;
}

/// 生成运行账本 owner；章节生成服务可持有克隆以写 generation run。
#[derive(Clone)]
pub(crate) struct GenerationApplication {
    database: DbHandle,
    task_queue: Arc<PersistentTaskQueue>,
    terminal: GenerationTerminalCoordinator,
}

struct RootGenerationTaskReservationInput<'a> {
    requested_by_user_id: i64,
    meta: &'a GenerationStartRequestMeta,
    business_key_json: Value,
    input_fingerprint_json: Value,
    task_kind: TaskKind,
    priority: TaskPriority,
    delivery_mode: GenerationDeliveryMode,
    batch_policy: Option<BatchCommitPolicy>,
    project_id: i64,
    task_scope: &'static str,
    target: GenerationTargetRef,
}

#[derive(Debug, Clone)]
pub struct GenerationTaskHistorySummaries {
    pub items: Vec<GenerationTaskSummary>,
    pub next_cursor: Option<GenerationTaskListCursor>,
}

/// 已完成权限校验的 generation task 句柄。
///
/// Checkpoint 和 public snapshot 仍由 generation owner 持有；Runtime/Host
/// 只能读取订阅所需的稳定投影，不能依赖数据库实体字段。
#[derive(Debug, Clone)]
pub struct AuthorizedGenerationTask {
    persisted: PersistedGenerationTask,
}

impl AuthorizedGenerationTask {
    pub fn public_snapshot(&self) -> Option<&GenerationTaskSnapshot> {
        self.persisted.public_snapshot.as_ref()
    }

    pub fn snapshot_expired(&self) -> bool {
        self.persisted
            .checkpoint
            .expires_at
            .is_some_and(|expires_at| expires_at <= Utc::now().naive_utc())
    }

    pub fn covers_through_sequence(&self) -> i64 {
        self.persisted.checkpoint.covers_through_sequence
    }
}

/// 统一 generation read/subscribe 的权限与 attempt 错误。
#[derive(Debug, thiserror::Error)]
pub enum GenerationTaskAccessError {
    #[error("generation task 不存在: {0}")]
    NotFound(String),
    #[error("generation task 无权访问: {0}")]
    Forbidden(String),
    #[error("generation task attempt 不匹配: expected={expected}, actual={actual}")]
    AttemptMismatch { expected: u32, actual: u32 },
    #[error("generation task 没有 live session，需要 snapshot 恢复")]
    SnapshotRequired,
    #[error("generation request invalid: {0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Persistence(#[from] GenerationTaskPersistenceError),
    #[error(transparent)]
    Session(#[from] GenerationSessionError),
    #[error(transparent)]
    Terminal(#[from] GenerationTerminalError),
    #[error("generation queue operation failed: {0}")]
    Queue(#[source] AppError),
    #[error("generation project authorization failed: {0}")]
    Project(#[source] AppError),
}

impl GenerationTaskAccessError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "generation_task_not_found",
            Self::Forbidden(_) => "generation_task_forbidden",
            Self::AttemptMismatch { .. } => "generation_attempt_mismatch",
            Self::SnapshotRequired => "generation_snapshot_required",
            Self::InvalidArgument(_) => "generation_request_invalid",
            Self::Persistence(error) => error.code(),
            Self::Session(error) => error.code(),
            Self::Terminal(error) => error.code(),
            Self::Queue(_) => "generation_persist_failed",
            Self::Project(AppError::Forbidden(_)) => "generation_task_forbidden",
            Self::Project(AppError::Unauthorized(_)) => "generation_task_unauthenticated",
            Self::Project(AppError::NotFound(_)) => "generation_task_not_found",
            Self::Project(_) => "generation_persist_failed",
        }
    }
}

/// 已取得 generation queue execution permit 的运行上下文。
pub(crate) struct GenerationTaskExecution {
    pub(crate) session: Arc<GenerationTaskSession>,
    pub(crate) publisher: SessionGenerationEventPublisher,
    pub(crate) checkpoint_writer: GenerationCheckpointWriter,
    pub(crate) terminal: GenerationTerminalCoordinator,
    pub(crate) permit: TaskPermit,
}

impl GenerationTaskExecution {
    pub(crate) async fn complete_with_result(
        self,
        operation_key: &str,
        result_ref: GenerationResultRef,
        summary: String,
        artifacts: Vec<PreparedArtifactEvent>,
        batch_summary: Option<BatchSummary>,
    ) -> Result<(), String> {
        let snapshot = self.session.snapshot();
        let final_text_hashes = snapshot_text_hashes(&snapshot.streams);
        let final_usage = snapshot.usage;
        let failure_batch_summary = batch_summary.clone();
        let failure_artifacts = artifacts.clone();
        let completion = self
            .terminal
            .complete_with_artifacts(&self.session, artifacts, move |_tx| {
                Box::pin(async move {
                    Ok(CompletionDescriptor {
                        result_ref: Some(result_ref),
                        final_text_hashes,
                        final_usage,
                        finish_reason: Some(GenerationFinishReason::Stop),
                        summary: Some(summary),
                        batch_summary,
                    })
                }) as BoxFuture<'_, Result<CompletionDescriptor, AppError>>
            })
            .await;
        if let Err(error) = completion {
            return Err(self
                .fail_with_batch_summary(
                    crate::generation::generation_public_error(error.code(), operation_key, error),
                    failure_batch_summary,
                    failure_artifacts,
                )
                .await);
        }
        self.finish();
        Ok(())
    }

    pub(crate) fn finish(self) {
        settle_generation_permit(
            self.permit,
            self.session.snapshot().status,
            "generation task execution 已结束".to_string(),
        );
    }

    pub(crate) async fn fail(self, error: GenerationError) -> String {
        self.fail_with_batch_summary(error, None, Vec::new()).await
    }

    pub(crate) async fn fail_with_batch_summary(
        self,
        error: GenerationError,
        batch_summary: Option<BatchSummary>,
        artifacts: Vec<PreparedArtifactEvent>,
    ) -> String {
        let Self {
            session,
            terminal,
            permit,
            ..
        } = self;
        let session_status = session.snapshot().status;
        let terminal_result = match session_status {
            GenerationTaskStatus::Cancelling => {
                terminal
                    .cancel(&session, None, batch_summary.clone(), artifacts)
                    .await
            }
            _ if generation_failure_is_cancelled_or_interrupted(session_status, &error) => {
                terminal
                    .interrupt(
                        &session,
                        InterruptionReason::SessionOwnerLost,
                        Some(error.message.clone()),
                        batch_summary.clone(),
                        artifacts,
                    )
                    .await
            }
            _ => {
                terminal
                    .fail(&session, error.clone(), batch_summary, artifacts)
                    .await
            }
        };
        let final_status = session.snapshot().status;
        settle_generation_permit(permit, final_status, error.message.clone());

        let mut message = error.message;
        if let Err(terminal_error) = terminal_result {
            message.push_str(&format!("; generation 终态写入失败: {terminal_error}"));
        }
        message
    }
}

pub(crate) async fn fail_generation_workflow_execution(
    orchestrator: &OrchestratorContext,
    execution: GenerationTaskExecution,
    workflow_run_id: Option<i64>,
    error: GenerationError,
) -> Result<(), String> {
    let cancelled_or_interrupted =
        generation_failure_is_cancelled_or_interrupted(execution.session.snapshot().status, &error);
    let workflow_error = if let Some(workflow_run_id) = workflow_run_id {
        let result = if cancelled_or_interrupted {
            orchestrator
                .cancel_workflow_run_status_validated(workflow_run_id, &error.message)
                .await
        } else {
            orchestrator
                .fail_workflow_run_validated(workflow_run_id, &error.message)
                .await
        };
        result.err().map(|error| (workflow_run_id, error))
    } else {
        None
    };
    let mut message = execution.fail(error).await;
    if let Some((workflow_run_id, workflow_error)) = workflow_error {
        message.push_str(&format!(
            "; generation workflow {workflow_run_id} 终态写入失败: {workflow_error}"
        ));
    }
    Err(message)
}

fn settle_generation_permit(permit: TaskPermit, status: GenerationTaskStatus, message: String) {
    match status {
        GenerationTaskStatus::Completed => permit.complete(),
        GenerationTaskStatus::Cancelled => permit.cancel(),
        GenerationTaskStatus::Interrupted => permit.interrupt(Some(message)),
        GenerationTaskStatus::Failed => permit.fail(message),
        status => permit.fail(format!(
            "{message}; generation lifecycle 未进入终态: {status:?}"
        )),
    }
}

fn snapshot_text_hashes(streams: &[TextStreamSnapshot]) -> Vec<StreamTextHash> {
    streams
        .iter()
        .map(|stream| StreamTextHash {
            stream_id: stream.stream_id.clone(),
            utf8_bytes: stream.next_offset,
            sha256: stream.sha256.clone(),
        })
        .collect()
}

#[cfg(test)]
mod completion_hash_tests {
    use super::*;

    fn settled_permit_status(status: GenerationTaskStatus) -> TaskStatus {
        let settled = Arc::new(std::sync::Mutex::new(None));
        let observed = Arc::clone(&settled);
        let permit = TaskPermit::new(
            "00000000-0000-4000-8000-000000000001".to_string(),
            TaskKind::LlmCall,
            TaskPriority::High,
            Box::new(move |status| {
                *observed.lock().expect("permit status mutex poisoned") = Some(status);
            }),
        );

        settle_generation_permit(permit, status, "terminal summary".to_string());

        Arc::try_unwrap(settled)
            .expect("permit status observer still shared")
            .into_inner()
            .expect("permit status mutex poisoned")
            .expect("permit was not settled")
    }

    #[test]
    fn completion_hashes_cover_every_snapshot_stream() {
        let streams = vec![
            TextStreamSnapshot {
                stream_id: "raw_json".to_string(),
                text: "{}".to_string(),
                next_offset: 2,
                sha256: "a".repeat(64),
            },
            TextStreamSnapshot {
                stream_id: "notes".to_string(),
                text: "ok".to_string(),
                next_offset: 2,
                sha256: "b".repeat(64),
            },
        ];

        let hashes = snapshot_text_hashes(&streams);

        assert_eq!(hashes.len(), streams.len());
        for (hash, stream) in hashes.iter().zip(&streams) {
            assert_eq!(hash.stream_id, stream.stream_id);
            assert_eq!(hash.utf8_bytes, stream.next_offset);
            assert_eq!(hash.sha256, stream.sha256);
        }
    }

    #[test]
    fn generation_permit_settlement_mirrors_terminal_status() {
        assert!(matches!(
            settled_permit_status(GenerationTaskStatus::Completed),
            TaskStatus::Completed
        ));
        assert!(matches!(
            settled_permit_status(GenerationTaskStatus::Failed),
            TaskStatus::Failed { error } if error == "terminal summary"
        ));
        assert!(matches!(
            settled_permit_status(GenerationTaskStatus::Cancelled),
            TaskStatus::Cancelled
        ));
        assert!(matches!(
            settled_permit_status(GenerationTaskStatus::Interrupted),
            TaskStatus::Interrupted { reason: Some(reason) } if reason == "terminal summary"
        ));
        assert!(matches!(
            settled_permit_status(GenerationTaskStatus::Running),
            TaskStatus::Failed { error } if error.contains("未进入终态: Running")
        ));
    }
}

impl GenerationApplication {
    pub(crate) fn new(database: DbHandle, task_queue: Arc<PersistentTaskQueue>) -> Self {
        let registry = Arc::new(
            GenerationSessionRegistry::new(GenerationSessionConfig::default())
                .expect("default generation session config must be valid"),
        );
        let terminal = GenerationTerminalCoordinator::new(database.clone(), registry);
        Self {
            database,
            task_queue,
            terminal,
        }
    }

    fn connection(&self) -> &sea_orm::DatabaseConnection {
        self.database.connection()
    }

    async fn initialize_reserved_task(
        &self,
        reservation: a4n_db::generation::GenerationTaskReservation,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let a4n_db::generation::GenerationTaskReservation {
            created,
            task,
            queue_task,
        } = reservation;
        let snapshot = required_public_snapshot(task.clone())
            .map_err(|error| GenerationTaskAccessError::InvalidArgument(error.to_string()))?;
        let handle = generation_task_handle(&snapshot);
        let queue_pending = matches!(queue_task.status, TaskStatus::Pending);

        if queue_pending && let Err(error) = self.task_queue.register_persisted(queue_task) {
            self.terminate_start_failure(
                &task.checkpoint.task_id,
                task.checkpoint.attempt,
                false,
                error.to_string(),
            )
            .await;
            return Err(GenerationTaskAccessError::Queue(error));
        }

        if snapshot.status.is_terminal()
            || self.session(&snapshot.task_id, snapshot.attempt)?.is_some()
        {
            return Ok(GenerationTaskStart {
                handle,
                should_run: false,
            });
        }

        // A duplicate request can resume only a still-reserved attempt. Running
        // and cancelling attempts belong to an existing executor/recovery path.
        if !created && snapshot.status != GenerationTaskStatus::Reserved {
            return Ok(GenerationTaskStart {
                handle,
                should_run: false,
            });
        }

        let admission = match self.terminal.registry().try_admit() {
            Ok(admission) => admission,
            Err(error) => {
                self.terminate_start_failure(
                    &task.checkpoint.task_id,
                    task.checkpoint.attempt,
                    queue_pending,
                    error.to_string(),
                )
                .await;
                return Err(GenerationTaskAccessError::Session(error));
            }
        };
        let started = a4n_db::generation::started_event_from_checkpoint(&task.checkpoint)?;
        let started_event = GenerationTaskEvent {
            schema_version: GENERATION_EVENT_SCHEMA_VERSION,
            task_id: snapshot.task_id.clone(),
            attempt: snapshot.attempt,
            sequence: snapshot.covers_through_sequence,
            emitted_at_ms: snapshot.updated_at_ms,
            event: GenerationEvent::Started(started),
        };
        let reserved_through_sequence =
            match u64::try_from(task.checkpoint.reserved_through_sequence) {
                Ok(value) => value,
                Err(_) => {
                    let error = GenerationSessionError::InvalidSequenceLease {
                        current: snapshot.covers_through_sequence,
                        next: u64::MAX,
                    };
                    self.terminate_start_failure(
                        &task.checkpoint.task_id,
                        task.checkpoint.attempt,
                        queue_pending,
                        error.to_string(),
                    )
                    .await;
                    return Err(GenerationTaskAccessError::Session(error));
                }
            };
        let session = match GenerationTaskSession::from_snapshot(
            snapshot.clone(),
            vec![started_event],
            task.checkpoint.checkpoint_revision,
            reserved_through_sequence,
            snapshot.updated_at_ms,
            admission,
        ) {
            Ok(session) => session,
            Err(error) => {
                self.terminate_start_failure(
                    &task.checkpoint.task_id,
                    task.checkpoint.attempt,
                    queue_pending,
                    error.to_string(),
                )
                .await;
                return Err(GenerationTaskAccessError::Session(error));
            }
        };
        let registered = match self.terminal.registry().insert_or_get(Arc::clone(&session)) {
            Ok(registered) => registered,
            Err(error) => {
                self.terminate_start_failure(
                    &task.checkpoint.task_id,
                    task.checkpoint.attempt,
                    queue_pending,
                    error.to_string(),
                )
                .await;
                return Err(GenerationTaskAccessError::Session(error));
            }
        };
        if !Arc::ptr_eq(&registered, &session) {
            return Ok(GenerationTaskStart {
                handle,
                should_run: false,
            });
        }

        Ok(GenerationTaskStart {
            handle,
            should_run: true,
        })
    }

    async fn terminate_start_failure(
        &self,
        task_id: &str,
        attempt: i32,
        queue_registered: bool,
        summary: String,
    ) {
        if let Ok(attempt) = u32::try_from(attempt) {
            let _ = self
                .terminal
                .interrupt_persisted_task(
                    task_id,
                    attempt,
                    a4n_service_protocol::InterruptionReason::SessionOwnerLost,
                    Some(summary),
                    None,
                )
                .await;
        }
        if queue_registered {
            let _ = self.task_queue.project_generation_terminal(
                task_id,
                TaskStatus::Interrupted {
                    reason: Some("generation start failed".to_string()),
                },
            );
        }
    }

    async fn reserve_root_generation_task(
        &self,
        input: RootGenerationTaskReservationInput<'_>,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let RootGenerationTaskReservationInput {
            requested_by_user_id,
            meta,
            business_key_json,
            input_fingerprint_json,
            task_kind,
            priority,
            delivery_mode,
            batch_policy,
            project_id,
            task_scope,
            target,
        } = input;
        let reservation = a4n_db::generation::reserve_generation_task(
            self.connection(),
            ReserveGenerationTaskInput {
                requested_by_user_id,
                start_request_id: meta.start_request_id.clone(),
                business_key_json,
                input_fingerprint_json,
                task_kind,
                priority,
                delivery_mode,
                batch_policy,
                project_id: Some(project_id),
                root_project_id: None,
                source_snapshot_id: None,
                task_scope: Some(task_scope.to_string()),
                workflow_run_id: None,
                generation_run_id: None,
                target: Some(target),
                root_task_id: None,
                parent_task_id: None,
                item_key: None,
                retry_of_task_id: meta.retry_of_task_id.clone(),
            },
        )
        .await
        .map_err(GenerationTaskAccessError::Persistence)?;
        self.initialize_reserved_task(reservation).await
    }

    pub(crate) async fn start_chapter_generation_task(
        &self,
        user_id: i64,
        input: &StartChapterGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let fingerprint = chapter_generation_input_fingerprint(input);
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "chapter_generation",
                "project_id": input.project_id,
                "chapter_number": input.chapter_number,
            }),
            input_fingerprint_json: fingerprint,
            task_kind: TaskKind::LlmCall,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id: input.project_id,
            task_scope: "chapter_generation",
            target: GenerationTargetRef {
                kind: "chapter".to_string(),
                id: format!("{}:{}", input.project_id, input.chapter_number),
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_inspiration_generation_task(
        &self,
        user_id: i64,
        input: &StartInspirationGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let fingerprint = inspiration_generation_input_fingerprint(input);
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "inspiration_generation",
                "project_id": input.project_id,
            }),
            input_fingerprint_json: fingerprint,
            task_kind: TaskKind::LlmCall,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id: input.project_id,
            task_scope: "inspiration_generation",
            target: GenerationTargetRef {
                kind: "inspiration_conversation".to_string(),
                id: input.project_id.to_string(),
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_outline_generation_task(
        &self,
        user_id: i64,
        input: &StartOutlineGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let operation_kind = outline_generation_operation_kind(&input.operation);
        let target_id = outline_generation_target_id(input.project_id, &input.operation);
        let fingerprint = outline_generation_input_fingerprint(input);
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "outline_generation",
                "project_id": input.project_id,
                "operation_kind": operation_kind,
            }),
            input_fingerprint_json: fingerprint,
            task_kind: TaskKind::LlmCall,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id: input.project_id,
            task_scope: "outline_generation",
            target: GenerationTargetRef {
                kind: format!("outline_{operation_kind}"),
                id: target_id,
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_portrait_generation_task(
        &self,
        user_id: i64,
        input: &StartPortraitGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let operation_kind = portrait_generation_operation_kind(&input.operation);
        let fingerprint = portrait_generation_input_fingerprint(input);
        let batch_policy = matches!(
            &input.operation,
            PortraitGenerationOperation::GenerateMissing { .. }
        )
        .then_some(BatchCommitPolicy::BestEffort);
        let target_id = portrait_generation_target_id(input.project_id, &input.operation);
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "portrait_generation",
                "project_id": input.project_id,
                "operation_kind": operation_kind,
            }),
            input_fingerprint_json: fingerprint,
            task_kind: TaskKind::ImageGeneration,
            priority: TaskPriority::Medium,
            delivery_mode: GenerationDeliveryMode::StageOnly,
            batch_policy,
            project_id: input.project_id,
            task_scope: "portrait_generation",
            target: GenerationTargetRef {
                kind: format!("portrait_{operation_kind}"),
                id: target_id,
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_image_generation_task(
        &self,
        user_id: i64,
        input: &StartImageGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let operation_kind = image_generation_operation_kind(&input.operation);
        let fingerprint = image_generation_input_fingerprint(input);
        let batch_policy = matches!(&input.operation, ImageGenerationOperation::Batch { .. })
            .then_some(BatchCommitPolicy::BestEffort);
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "image_generation",
                "project_id": input.project_id,
                "operation_kind": operation_kind,
            }),
            input_fingerprint_json: fingerprint,
            task_kind: TaskKind::ImageGeneration,
            priority: TaskPriority::Medium,
            delivery_mode: GenerationDeliveryMode::StageOnly,
            batch_policy,
            project_id: input.project_id,
            task_scope: "image_generation",
            target: GenerationTargetRef {
                kind: format!("image_{operation_kind}"),
                id: image_generation_target_id(input.project_id, &input.operation),
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_video_generation_task(
        &self,
        user_id: i64,
        input: &StartVideoGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let operation_kind = input.operation.kind();
        let target_id = video_generation_target_id(input.project_id, &input.operation)
            .map_err(|error| GenerationTaskAccessError::InvalidArgument(error.to_string()))?;
        let fingerprint = video_generation_input_fingerprint(input);
        let prompt_operation = input.operation.is_prompt_generation();
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: video_generation_business_key(
                input.project_id,
                operation_kind,
                &target_id,
            ),
            input_fingerprint_json: fingerprint,
            task_kind: if prompt_operation {
                TaskKind::LlmCall
            } else {
                TaskKind::VideoGeneration
            },
            priority: if prompt_operation {
                TaskPriority::High
            } else {
                TaskPriority::Medium
            },
            delivery_mode: if prompt_operation {
                GenerationDeliveryMode::Stream
            } else {
                GenerationDeliveryMode::StageOnly
            },
            batch_policy: match &input.operation {
                VideoGenerationOperation::ChapterBatch { .. }
                | VideoGenerationOperation::PromptFrames { .. }
                | VideoGenerationOperation::PromptVideos { .. } => {
                    Some(BatchCommitPolicy::BestEffort)
                }
                _ => None,
            },
            project_id: input.project_id,
            task_scope: if prompt_operation {
                "video_prompt_generation"
            } else {
                "video_generation"
            },
            target: GenerationTargetRef {
                kind: format!("video_{operation_kind}"),
                id: target_id,
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_blueprint_generation_task(
        &self,
        user_id: i64,
        input: &StartBlueprintGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let fingerprint = blueprint_generation_input_fingerprint(input);
        self.start_blueprint_mutation_task(
            user_id,
            &input.meta,
            input.project_id,
            fingerprint,
            BLUEPRINT_GENERATION_KIND,
        )
        .await
    }

    pub(crate) async fn start_blueprint_refinement_task(
        &self,
        user_id: i64,
        input: &StartBlueprintRefinementInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let fingerprint = blueprint_refinement_input_fingerprint(input);
        self.start_blueprint_mutation_task(
            user_id,
            &input.meta,
            input.project_id,
            fingerprint,
            BLUEPRINT_REFINEMENT_TASK_KIND,
        )
        .await
    }

    async fn start_blueprint_mutation_task(
        &self,
        user_id: i64,
        meta: &GenerationStartRequestMeta,
        project_id: i64,
        fingerprint: Value,
        task_scope: &'static str,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "blueprint_mutation",
                "project_id": project_id,
                "operation_mode": "blueprint",
            }),
            input_fingerprint_json: fingerprint,
            task_kind: TaskKind::LlmCall,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id,
            task_scope,
            target: GenerationTargetRef {
                kind: task_scope.to_string(),
                id: project_id.to_string(),
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_optimize_chat_generation_task(
        &self,
        user_id: i64,
        input: &StartOptimizeChatGenerationInput,
        project_id: i64,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let fingerprint = optimize_chat_generation_input_fingerprint(input);
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "optimization_chat",
                "session_id": input.request.session_id,
            }),
            input_fingerprint_json: fingerprint,
            task_kind: TaskKind::LlmCall,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id,
            task_scope: "optimization_chat",
            target: GenerationTargetRef {
                kind: "optimization_chat".to_string(),
                id: input.request.session_id.to_string(),
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_optimize_analysis_generation_task(
        &self,
        user_id: i64,
        input: &StartOptimizeAnalysisGenerationInput,
        project_id: i64,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let input_fingerprint_json = optimize_analysis_generation_input_fingerprint(input);
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "optimization_analysis",
                "session_id": input.session_id,
            }),
            input_fingerprint_json,
            task_kind: TaskKind::LlmCall,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id,
            task_scope: "optimization_analysis",
            target: GenerationTargetRef {
                kind: "optimization_analysis".to_string(),
                id: input.session_id.to_string(),
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn start_optimize_patch_generation_task(
        &self,
        user_id: i64,
        input: &StartOptimizePatchGenerationInput,
        project_id: i64,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let input_fingerprint_json = optimize_patch_generation_input_fingerprint(input);
        self.reserve_root_generation_task(RootGenerationTaskReservationInput {
            requested_by_user_id: user_id,
            meta: &input.meta,
            business_key_json: serde_json::json!({
                "version": 1,
                "task_kind": "optimization_patch",
                "session_id": input.request.session_id,
            }),
            input_fingerprint_json,
            task_kind: TaskKind::LlmCall,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id,
            task_scope: "optimization_patch",
            target: GenerationTargetRef {
                kind: "optimization_patch".to_string(),
                id: input.request.session_id.to_string(),
                revision: None,
            },
        })
        .await
    }

    pub(crate) async fn acquire_task_execution(
        &self,
        handle: &GenerationTaskHandle,
    ) -> Result<Option<GenerationTaskExecution>, GenerationTaskAccessError> {
        let session = self
            .session(&handle.task_id, handle.attempt)?
            .ok_or(GenerationTaskAccessError::SnapshotRequired)?;
        let Some(permit) = self
            .task_queue
            .acquire_generation_task(&handle.task_id)
            .await
            .map_err(GenerationTaskAccessError::Queue)?
        else {
            return Ok(None);
        };
        match self.terminal.begin_running(&session).await {
            Ok(GenerationTerminalWriteOutcome::Persisted)
            | Ok(GenerationTerminalWriteOutcome::RebuiltFromPersistedSnapshot) => {
                let publisher = SessionGenerationEventPublisher::new(
                    session.clone(),
                    Arc::new(DatabaseGenerationSequenceLeaseReplenisher::new(
                        self.database.shared_connection(),
                    )),
                );
                Ok(Some(GenerationTaskExecution {
                    session,
                    publisher,
                    checkpoint_writer: GenerationCheckpointWriter::new(
                        self.database.shared_connection(),
                    ),
                    terminal: self.terminal.clone(),
                    permit,
                }))
            }
            Ok(GenerationTerminalWriteOutcome::AlreadyTerminal)
            | Ok(GenerationTerminalWriteOutcome::NoLiveSession) => {
                settle_generation_permit(
                    permit,
                    session.snapshot().status,
                    "generation task 在取得执行权前已结束".to_string(),
                );
                Ok(None)
            }
            Err(error) => {
                permit.fail(error.to_string());
                Err(GenerationTaskAccessError::Terminal(error))
            }
        }
    }

    async fn load_task_checkpoint(
        &self,
        task_id: &str,
    ) -> Result<
        a4n_db_models::models::generation::generation_task_checkpoint::Model,
        GenerationTaskAccessError,
    > {
        a4n_db::generation::load_generation_task_checkpoint(self.connection(), task_id)
            .await
            .map_err(|error| match error {
                GenerationTaskPersistenceError::NotFound(message) => {
                    GenerationTaskAccessError::NotFound(message)
                }
                other => GenerationTaskAccessError::Persistence(other),
            })
    }

    async fn list_active_task_summaries(
        &self,
        user_id: i64,
        project_id: Option<i64>,
    ) -> Result<Vec<GenerationTaskSummary>, GenerationTaskAccessError> {
        let page = a4n_db::generation::list_active_generation_task_checkpoints(
            self.connection(),
            user_id,
            project_id,
        )
        .await
        .map_err(GenerationTaskAccessError::Persistence)?;
        summarize_checkpoint_page(page)
    }

    async fn list_history_task_summaries(
        &self,
        user_id: i64,
        project_id: Option<i64>,
        limit: u32,
        cursor: Option<GenerationTaskListCursor>,
    ) -> Result<GenerationTaskHistorySummaries, GenerationTaskAccessError> {
        let page = a4n_db::generation::list_generation_task_history_checkpoints(
            self.connection(),
            user_id,
            project_id,
            limit,
            cursor,
        )
        .await
        .map_err(GenerationTaskAccessError::Persistence)?;
        let next_cursor = if page.has_more {
            page.items
                .last()
                .map(|checkpoint| GenerationTaskListCursor {
                    updated_at_ms: DateTime::<Utc>::from_naive_utc_and_offset(
                        checkpoint.updated_at,
                        Utc,
                    )
                    .timestamp_millis(),
                    task_id: checkpoint.task_id.clone(),
                })
        } else {
            None
        };
        let items = summarize_checkpoint_page(page)?;
        Ok(GenerationTaskHistorySummaries { items, next_cursor })
    }

    pub(crate) async fn record_completed_generation_run(
        &self,
        input: RecordCompletedGenerationRunInput,
    ) -> Result<generation_run::Model, AppError> {
        a4n_db::generation::record_completed_generation_run(self.connection(), input)
            .await
            .map_err(generation_error)
    }

    async fn record_completed_generation_run_with_checkpoint(
        &self,
        input: RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<generation_run::Model, AppError> {
        a4n_db::generation::record_completed_run_with_checkpoint_transaction(
            self.connection(),
            input,
        )
        .await
        .map_err(generation_error)
    }

    async fn attach_generation_runs_to_workflow_for_target(
        &self,
        input: AttachGenerationRunsToWorkflowInput,
    ) -> Result<usize, AppError> {
        a4n_db::generation::attach_runs_to_workflow_for_target(self.connection(), input)
            .await
            .map_err(generation_error)
    }

    pub(crate) async fn record_llm_call_trace(
        &self,
        input: RecordLlmCallTraceInput,
    ) -> Result<llm_call_trace::Model, AppError> {
        a4n_db::generation::record_llm_call_trace(self.connection(), input)
            .await
            .map_err(generation_error)
    }

    pub(super) async fn complete_generation_runs_for_target(
        &self,
        target_table: &str,
        target_id: i64,
        response_summary_json: Option<Value>,
    ) -> Result<usize, AppError> {
        a4n_db::generation::complete_runs_for_target(
            self.connection(),
            target_table,
            target_id,
            response_summary_json,
        )
        .await
        .map_err(generation_error)
    }

    pub(super) async fn fail_generation_runs_for_target(
        &self,
        target_table: &str,
        target_id: i64,
        error_message: String,
        response_summary_json: Option<Value>,
    ) -> Result<usize, AppError> {
        a4n_db::generation::fail_runs_for_target(
            self.connection(),
            target_table,
            target_id,
            error_message,
            response_summary_json,
        )
        .await
        .map_err(generation_error)
    }

    pub(super) async fn cancel_generation_runs_for_target(
        &self,
        target_table: &str,
        target_id: i64,
        message: String,
    ) -> Result<usize, AppError> {
        a4n_db::generation::cancel_runs_for_target(
            self.connection(),
            target_table,
            target_id,
            message,
        )
        .await
        .map_err(generation_error)
    }

    pub(super) async fn cancel_generation_runs_for_targets(
        &self,
        target_table: &str,
        target_ids: &[i64],
        message: String,
    ) -> Result<Vec<(i64, usize)>, AppError> {
        a4n_db::generation::cancel_runs_for_targets(
            self.connection(),
            target_table,
            target_ids,
            message,
        )
        .await
        .map_err(generation_error)
    }

    async fn generation_run_exists_for_target_output(
        &self,
        target_table: &str,
        target_id: i64,
        step_key: &str,
        response_output: &str,
    ) -> Result<bool, AppError> {
        a4n_db::generation::generation_run_exists_for_target_output(
            self.connection(),
            target_table,
            target_id,
            step_key,
            response_output,
        )
        .await
        .map_err(generation_error)
    }

    fn session(
        &self,
        task_id: &str,
        attempt: u32,
    ) -> Result<Option<Arc<GenerationTaskSession>>, GenerationTerminalError> {
        self.terminal
            .registry()
            .get(task_id, attempt)
            .map_err(Into::into)
    }

    fn session_for_authorized_task(
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
    async fn validate_video_generation_start(
        &self,
        user_id: i64,
        input: &StartVideoGenerationInput,
    ) -> Result<(), AppError> {
        if input.project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "project_id 必须大于 0".to_string(),
            ));
        }
        self.verify_project_permission(
            input.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        match &input.operation {
            VideoGenerationOperation::Chapter {
                chapter_id,
                prompt_id,
                config_id,
            } => {
                if *chapter_id <= 0 || *prompt_id <= 0 || config_id.is_some_and(|id| id <= 0) {
                    return Err(AppError::InvalidArgument(
                        "章节视频的 chapter_id、prompt_id、config_id 必须为正数".to_string(),
                    ));
                }
            }
            VideoGenerationOperation::ChapterBatch {
                chapter_id,
                prompt_ids,
                config_id,
            } => {
                if *chapter_id <= 0 || prompt_ids.is_empty() || config_id.is_some_and(|id| id <= 0)
                {
                    return Err(AppError::InvalidArgument(
                        "批量视频必须包含 chapter_id 和 prompt_ids".to_string(),
                    ));
                }
                if prompt_ids.iter().any(|id| *id <= 0) {
                    return Err(AppError::InvalidArgument(
                        "批量视频 prompt_id 必须为正数".to_string(),
                    ));
                }
                let mut unique = prompt_ids.clone();
                unique.sort_unstable();
                unique.dedup();
                if unique.len() != prompt_ids.len() {
                    return Err(AppError::InvalidArgument(
                        "批量视频 prompt_ids 不能重复".to_string(),
                    ));
                }
            }
            VideoGenerationOperation::ReferenceMedia { chapter_id, input } => {
                if *chapter_id <= 0 {
                    return Err(AppError::InvalidArgument(
                        "参考媒体视频必须包含 chapter_id".to_string(),
                    ));
                }
                if input.config_id.is_some_and(|id| id <= 0) {
                    return Err(AppError::InvalidArgument(
                        "参考媒体视频的 config_id 无效".to_string(),
                    ));
                }
                validate_reference_media_start_input(input).map_err(AppError::InvalidArgument)?;
            }
            VideoGenerationOperation::PromptSegmentPlan {
                chapter_id,
                options,
            } => validate_video_prompt_generation_start(
                *chapter_id,
                options,
                crate::video::validate_video_segment_plan_generation_options,
            )?,
            VideoGenerationOperation::PromptInsertedSegment { options, .. } => {
                crate::video::validate_video_inserted_segment_generation_options(options)?
            }
            VideoGenerationOperation::PromptFrames {
                chapter_id,
                options,
            } => validate_video_prompt_generation_start(
                *chapter_id,
                options,
                crate::video::validate_video_frame_prompt_generation_options,
            )?,
            VideoGenerationOperation::PromptVideos {
                chapter_id,
                options,
            } => validate_video_prompt_generation_start(
                *chapter_id,
                options,
                crate::video::validate_video_final_prompt_generation_options,
            )?,
        }
        Ok(())
    }

    pub(crate) async fn start_chapter_generation_task(
        &self,
        user_id: i64,
        input: &StartChapterGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.validate_chapter_generation_application_access(
            user_id,
            input.project_id,
            input.chapter_number,
            "生成章节正文",
        )
        .await
        .map_err(GenerationTaskAccessError::InvalidArgument)?;
        self.validate_chapter_generation_plan_input(
            input.project_id,
            input.chapter_number,
            input.writing_notes.as_deref(),
        )
        .await
        .map_err(GenerationTaskAccessError::InvalidArgument)?;
        self.validate_chapter_generation_client_rag_input(input)
            .await
            .map_err(GenerationTaskAccessError::InvalidArgument)?;
        self.generation
            .start_chapter_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn start_inspiration_generation_task(
        &self,
        user_id: i64,
        input: &StartInspirationGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        crate::inspiration::required_message_content(&input.message)
            .map_err(|error| GenerationTaskAccessError::InvalidArgument(error.to_string()))?;
        self.verify_project_permission(
            input.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await
        .map_err(GenerationTaskAccessError::Project)?;
        self.ensure_text_source_project(input.project_id, "灵感对话")
            .await
            .map_err(GenerationTaskAccessError::InvalidArgument)?;
        self.generation
            .start_inspiration_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn start_outline_generation_task(
        &self,
        user_id: i64,
        input: &StartOutlineGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.validate_outline_generation_start(user_id, input)
            .await
            .map_err(map_outline_generation_start_error)?;
        self.generation
            .start_outline_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn start_portrait_generation_task(
        &self,
        user_id: i64,
        input: &StartPortraitGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.validate_portrait_generation_start(user_id, input)
            .await
            .map_err(map_portrait_generation_start_error)?;
        self.generation
            .start_portrait_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn start_image_generation_task(
        &self,
        user_id: i64,
        input: &StartImageGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.validate_image_generation_start(user_id, input)
            .await
            .map_err(map_image_generation_start_error)?;
        self.generation
            .start_image_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn start_video_generation_task(
        &self,
        user_id: i64,
        input: &StartVideoGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.validate_video_generation_start(user_id, input)
            .await
            .map_err(map_video_generation_start_error)?;
        self.generation
            .start_video_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn start_blueprint_generation_task(
        &self,
        user_id: i64,
        input: &StartBlueprintGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        if input.title.trim().is_empty() || input.genre.trim().is_empty() {
            return Err(GenerationTaskAccessError::InvalidArgument(
                "蓝图标题和题材不能为空".to_string(),
            ));
        }
        if input.target_length <= 0 {
            return Err(GenerationTaskAccessError::InvalidArgument(
                "蓝图目标章节数必须大于 0".to_string(),
            ));
        }
        self.verify_project_permission(
            input.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await
        .map_err(GenerationTaskAccessError::Project)?;
        self.generation
            .start_blueprint_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn start_blueprint_refinement_task(
        &self,
        user_id: i64,
        input: &StartBlueprintRefinementInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.verify_project_permission(
            input.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await
        .map_err(GenerationTaskAccessError::Project)?;
        if self
            .novel_blueprint_for_project(input.project_id)
            .await
            .map_err(|error| GenerationTaskAccessError::InvalidArgument(error.to_string()))?
            .is_none()
        {
            return Err(GenerationTaskAccessError::InvalidArgument(
                "蓝图不存在".to_string(),
            ));
        }
        self.generation
            .start_blueprint_refinement_task(user_id, input)
            .await
    }

    pub(crate) async fn start_optimize_chat_generation_task(
        &self,
        user_id: i64,
        input: &StartOptimizeChatGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let session = self
            .validated_optimize_chat_generation_session(
                user_id,
                input.request.session_id,
                &input.request.message,
            )
            .await
            .map_err(map_optimize_generation_start_error)?;
        self.generation
            .start_optimize_chat_generation_task(user_id, input, session.project_id)
            .await
    }

    pub(crate) async fn start_optimize_analysis_generation_task(
        &self,
        user_id: i64,
        input: &StartOptimizeAnalysisGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let session = self
            .optimize_session_for_user(user_id, input.session_id)
            .await
            .map_err(map_optimize_generation_start_error)?;
        self.generation
            .start_optimize_analysis_generation_task(user_id, input, session.project_id)
            .await
    }

    pub(crate) async fn start_optimize_patch_generation_task(
        &self,
        user_id: i64,
        input: &StartOptimizePatchGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let session = self
            .optimize_session_for_user(user_id, input.request.session_id)
            .await
            .map_err(map_optimize_generation_start_error)?;
        self.generation
            .start_optimize_patch_generation_task(user_id, input, session.project_id)
            .await
    }

    pub(crate) async fn acquire_generation_task_execution(
        &self,
        handle: &GenerationTaskHandle,
    ) -> Result<Option<GenerationTaskExecution>, GenerationTaskAccessError> {
        self.generation.acquire_task_execution(handle).await
    }

    /// 读取 generation task 前统一检查 owner、attempt 和项目 Viewer 权限。
    pub(crate) async fn authorize_generation_task(
        &self,
        user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<AuthorizedGenerationTask, GenerationTaskAccessError> {
        let checkpoint = self.generation.load_task_checkpoint(task_id).await?;
        authorize_checkpoint(&checkpoint, user_id, attempt, self).await?;
        let persisted = a4n_db::generation::persisted_generation_task_from_checkpoint(checkpoint)
            .map_err(GenerationTaskAccessError::Persistence)?;
        Ok(AuthorizedGenerationTask { persisted })
    }

    /// 读取 summary 时只校验 checkpoint 规范列；损坏/过期 snapshot 仍返回可展示条目。
    pub(crate) async fn generation_task_summary_for_user(
        &self,
        user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<GenerationTaskSummary, GenerationTaskAccessError> {
        let checkpoint = self.generation.load_task_checkpoint(task_id).await?;
        authorize_checkpoint(&checkpoint, user_id, attempt, self).await?;
        generation_task_summary_from_checkpoint(
            &checkpoint,
            a4n_db::generation::generation_public_snapshot_for_summary(&checkpoint),
        )
    }

    pub(crate) async fn list_active_generation_task_summaries(
        &self,
        user_id: i64,
        project_id: Option<i64>,
    ) -> Result<Vec<GenerationTaskSummary>, GenerationTaskAccessError> {
        if let Some(project_id) = project_id {
            self.verify_project_permission(
                project_id,
                user_id,
                crate::collaboration::ProjectAccessRole::Viewer,
            )
            .await
            .map_err(GenerationTaskAccessError::Project)?;
        }
        self.generation
            .list_active_task_summaries(user_id, project_id)
            .await
    }

    pub(crate) async fn list_generation_task_history_summaries(
        &self,
        user_id: i64,
        project_id: Option<i64>,
        limit: u32,
        cursor: Option<GenerationTaskListCursor>,
    ) -> Result<GenerationTaskHistorySummaries, GenerationTaskAccessError> {
        if let Some(project_id) = project_id {
            self.verify_project_permission(
                project_id,
                user_id,
                crate::collaboration::ProjectAccessRole::Viewer,
            )
            .await
            .map_err(GenerationTaskAccessError::Project)?;
        }
        self.generation
            .list_history_task_summaries(user_id, project_id, limit, cursor)
            .await
    }

    /// 只从已授权 task 取得进程内 session；不存在 live session 时显式要求 snapshot。
    pub(crate) fn subscribe_generation_task(
        &self,
        task: &AuthorizedGenerationTask,
        after_sequence: Option<u64>,
    ) -> Result<GenerationSubscription, GenerationTaskAccessError> {
        let session = self
            .generation
            .session_for_authorized_task(&task.persisted)?
            .ok_or(GenerationTaskAccessError::SnapshotRequired)?;
        session.subscribe(after_sequence).map_err(Into::into)
    }

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
            self.database.connection(),
            project_id,
            operation,
        )
        .await
    }

    pub(crate) async fn restore_generation_user_lifecycle_gate(
        &self,
        user_id: i64,
    ) -> Result<(), GenerationLifecycleGateError> {
        a4n_db::generation::restore_user_lifecycle_gate(self.database.connection(), user_id).await
    }

    pub(crate) async fn complete_generation_project_reset_gate(
        &self,
        project_id: i64,
    ) -> Result<(), GenerationLifecycleGateError> {
        a4n_db::generation::complete_project_reset_gate(self.database.connection(), project_id)
            .await
    }

    pub(crate) async fn complete_generation_user_disable_gate(
        &self,
        user_id: i64,
    ) -> Result<(), GenerationLifecycleGateError> {
        a4n_db::generation::complete_user_disable_gate(self.database.connection(), user_id).await
    }

    pub(crate) async fn record_completed_generation_run(
        &self,
        input: RecordCompletedGenerationRunInput,
    ) -> Result<generation_run::Model, AppError> {
        self.generation.record_completed_generation_run(input).await
    }

    pub(crate) async fn record_completed_generation_run_with_checkpoint(
        &self,
        input: RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<generation_run::Model, AppError> {
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

#[async_trait::async_trait]
impl GenerationApplicationPort for OrchestratorContext {
    async fn request_generation_cancel(
        &self,
        requested_by_user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<GenerationTaskHandle, GenerationTerminalError> {
        OrchestratorContext::request_generation_cancel(self, requested_by_user_id, task_id, attempt)
            .await
    }

    async fn authorize_generation_task(
        &self,
        user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<AuthorizedGenerationTask, GenerationTaskAccessError> {
        OrchestratorContext::authorize_generation_task(self, user_id, task_id, attempt).await
    }

    async fn generation_task_summary_for_user(
        &self,
        user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<GenerationTaskSummary, GenerationTaskAccessError> {
        OrchestratorContext::generation_task_summary_for_user(self, user_id, task_id, attempt).await
    }

    async fn list_active_generation_task_summaries(
        &self,
        user_id: i64,
        project_id: Option<i64>,
    ) -> Result<Vec<GenerationTaskSummary>, GenerationTaskAccessError> {
        OrchestratorContext::list_active_generation_task_summaries(self, user_id, project_id).await
    }

    async fn list_generation_task_history_summaries(
        &self,
        user_id: i64,
        project_id: Option<i64>,
        limit: u32,
        cursor: Option<GenerationTaskListCursor>,
    ) -> Result<GenerationTaskHistorySummaries, GenerationTaskAccessError> {
        OrchestratorContext::list_generation_task_history_summaries(
            self, user_id, project_id, limit, cursor,
        )
        .await
    }

    fn subscribe_generation_task(
        &self,
        task: &AuthorizedGenerationTask,
        after_sequence: Option<u64>,
    ) -> Result<GenerationSubscription, GenerationTaskAccessError> {
        OrchestratorContext::subscribe_generation_task(self, task, after_sequence)
    }

    async fn begin_generation_project_lifecycle_gate(
        &self,
        project_id: i64,
        user_id: i64,
        operation: ProjectLifecycleOperation,
    ) -> Result<ProjectLifecycleGateClaim, GenerationLifecycleGateError> {
        OrchestratorContext::begin_generation_project_lifecycle_gate(
            self, project_id, user_id, operation,
        )
        .await
    }

    async fn begin_generation_user_disable_gate(
        &self,
        caller_user_id: i64,
        user_id: i64,
    ) -> Result<UserLifecycleGateClaim, GenerationLifecycleGateError> {
        OrchestratorContext::begin_generation_user_disable_gate(self, caller_user_id, user_id).await
    }

    async fn active_generation_tasks_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError> {
        OrchestratorContext::active_generation_tasks_for_project(self, project_id).await
    }

    async fn active_generation_tasks_for_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError> {
        OrchestratorContext::active_generation_tasks_for_user(self, user_id).await
    }

    async fn active_generation_tasks(
        &self,
    ) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError> {
        OrchestratorContext::active_generation_tasks(self).await
    }

    async fn expire_generation_snapshot_bodies(
        &self,
        limit: usize,
    ) -> Result<u64, GenerationTaskLifecycleError> {
        OrchestratorContext::expire_generation_snapshot_bodies(self, limit).await
    }

    async fn purge_expired_generation_task_trees(
        &self,
        limit: usize,
    ) -> Result<u64, GenerationTaskLifecycleError> {
        OrchestratorContext::purge_expired_generation_task_trees(self, limit).await
    }

    async fn pending_generation_lifecycle_operations(
        &self,
    ) -> Result<Vec<PendingLifecycleOperation>, GenerationLifecycleGateError> {
        OrchestratorContext::pending_generation_lifecycle_operations(self).await
    }

    async fn request_generation_destructive_cancel(
        &self,
        task: &GenerationLifecycleTask,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        OrchestratorContext::request_generation_destructive_cancel(self, task).await
    }

    async fn interrupt_generation_destructive_task(
        &self,
        task: &GenerationLifecycleTask,
        reason: InterruptionReason,
        summary: String,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        OrchestratorContext::interrupt_generation_destructive_task(self, task, reason, summary)
            .await
    }

    async fn restore_generation_project_lifecycle_gate(
        &self,
        project_id: i64,
        operation: ProjectLifecycleOperation,
    ) -> Result<(), GenerationLifecycleGateError> {
        OrchestratorContext::restore_generation_project_lifecycle_gate(self, project_id, operation)
            .await
    }

    async fn restore_generation_user_lifecycle_gate(
        &self,
        user_id: i64,
    ) -> Result<(), GenerationLifecycleGateError> {
        OrchestratorContext::restore_generation_user_lifecycle_gate(self, user_id).await
    }

    async fn complete_generation_project_reset_gate(
        &self,
        project_id: i64,
    ) -> Result<(), GenerationLifecycleGateError> {
        OrchestratorContext::complete_generation_project_reset_gate(self, project_id).await
    }

    async fn complete_generation_user_disable_gate(
        &self,
        user_id: i64,
    ) -> Result<(), GenerationLifecycleGateError> {
        OrchestratorContext::complete_generation_user_disable_gate(self, user_id).await
    }

    async fn delete_project_cascade_validated(&self, project_id: i64) -> Result<(), AppError> {
        OrchestratorContext::delete_project_cascade_validated(self, project_id).await
    }

    async fn reset_project_generated_data_validated(
        &self,
        project_id: i64,
    ) -> Result<crate::project::ProjectGeneratedDataResetApplicationResult, AppError> {
        OrchestratorContext::reset_project_generated_data_validated(self, project_id).await
    }
}

async fn authorize_checkpoint(
    checkpoint: &a4n_db_models::models::generation::generation_task_checkpoint::Model,
    user_id: i64,
    attempt: u32,
    context: &OrchestratorContext,
) -> Result<(), GenerationTaskAccessError> {
    if checkpoint.requested_by_user_id != user_id {
        return Err(GenerationTaskAccessError::Forbidden(
            checkpoint.task_id.clone(),
        ));
    }
    let actual_attempt = u32::try_from(checkpoint.attempt).map_err(|_| {
        GenerationTaskAccessError::Persistence(GenerationTaskPersistenceError::Corrupt(format!(
            "generation task {} attempt 超出 u32 范围",
            checkpoint.task_id
        )))
    })?;
    if actual_attempt != attempt {
        return Err(GenerationTaskAccessError::AttemptMismatch {
            expected: attempt,
            actual: actual_attempt,
        });
    }
    if let Some(project_id) = checkpoint.project_id.or(checkpoint.root_project_id) {
        context
            .verify_project_permission(
                project_id,
                user_id,
                crate::collaboration::ProjectAccessRole::Viewer,
            )
            .await
            .map_err(GenerationTaskAccessError::Project)?;
    }
    Ok(())
}

fn summarize_checkpoint_page(
    page: GenerationTaskCheckpointPage,
) -> Result<Vec<GenerationTaskSummary>, GenerationTaskAccessError> {
    page.items
        .iter()
        .map(|checkpoint| {
            generation_task_summary_from_checkpoint(
                checkpoint,
                a4n_db::generation::generation_public_snapshot_for_summary(checkpoint),
            )
        })
        .collect()
}

fn generation_task_summary_from_checkpoint(
    checkpoint: &a4n_db_models::models::generation::generation_task_checkpoint::Model,
    public_snapshot: GenerationPublicSnapshot,
) -> Result<GenerationTaskSummary, GenerationTaskAccessError> {
    let started = a4n_db::generation::started_event_from_checkpoint(checkpoint)
        .map_err(GenerationTaskAccessError::Persistence)?;
    let status: GenerationTaskStatus =
        serde_json::from_value(Value::String(checkpoint.status.clone())).map_err(|error| {
            GenerationTaskAccessError::Persistence(GenerationTaskPersistenceError::Corrupt(
                format!(
                    "generation task {} status 无效: {error}",
                    checkpoint.task_id
                ),
            ))
        })?;
    let attempt = u32::try_from(checkpoint.attempt).map_err(|_| {
        GenerationTaskAccessError::Persistence(GenerationTaskPersistenceError::Corrupt(format!(
            "generation task {} attempt 超出 u32 范围",
            checkpoint.task_id
        )))
    })?;
    if attempt == 0 {
        return Err(GenerationTaskAccessError::Persistence(
            GenerationTaskPersistenceError::Corrupt(format!(
                "generation task {} attempt 必须大于 0",
                checkpoint.task_id
            )),
        ));
    }
    let result_ref = match (
        checkpoint.result_kind.as_ref(),
        checkpoint.result_id.as_ref(),
        checkpoint.result_revision.as_ref(),
    ) {
        (None, None, None) => None,
        (Some(kind), Some(id), Some(revision)) if status == GenerationTaskStatus::Completed => {
            Some(a4n_service_protocol::GenerationResultRef {
                kind: kind.clone(),
                id: id.clone(),
                revision: revision.clone(),
            })
        }
        (Some(_), Some(_), Some(_)) => None,
        _ => {
            return Err(GenerationTaskAccessError::Persistence(
                GenerationTaskPersistenceError::Corrupt(format!(
                    "generation task {} result ref 规范列不成组",
                    checkpoint.task_id
                )),
            ));
        }
    };
    let (snapshot_state, terminal_event) = match public_snapshot {
        GenerationPublicSnapshot::Available(snapshot) => (
            GenerationSnapshotState::Available,
            snapshot
                .status
                .is_terminal()
                .then_some(snapshot.terminal_event)
                .flatten(),
        ),
        GenerationPublicSnapshot::Expired => (GenerationSnapshotState::Expired, None),
        GenerationPublicSnapshot::Corrupt => (GenerationSnapshotState::Corrupt, None),
    };
    let updated_at_ms =
        DateTime::<Utc>::from_naive_utc_and_offset(checkpoint.updated_at, Utc).timestamp_millis();
    if !(0..=a4n_service_protocol::JS_SAFE_INTEGER_I64).contains(&updated_at_ms) {
        return Err(GenerationTaskAccessError::Persistence(
            GenerationTaskPersistenceError::Corrupt(format!(
                "generation task {} updated_at 超出 JavaScript safe integer",
                checkpoint.task_id
            )),
        ));
    }
    Ok(GenerationTaskSummary {
        handle: GenerationTaskHandle {
            event_schema_version: GENERATION_EVENT_SCHEMA_VERSION,
            task_id: checkpoint.task_id.clone(),
            attempt,
            status,
        },
        started,
        updated_at_ms,
        result_ref,
        snapshot_state,
        terminal_event,
    })
}

fn generation_task_handle(snapshot: &GenerationTaskSnapshot) -> GenerationTaskHandle {
    GenerationTaskHandle {
        event_schema_version: GENERATION_EVENT_SCHEMA_VERSION,
        task_id: snapshot.task_id.clone(),
        attempt: snapshot.attempt,
        status: snapshot.status,
    }
}

fn chapter_generation_input_fingerprint(input: &StartChapterGenerationInput) -> Value {
    serde_json::json!({
        "version": 2,
        "task_kind": "chapter_generation",
        "project_id": input.project_id,
        "chapter_number": input.chapter_number,
        "writing_notes": input.writing_notes,
        "min_word_count": input.min_word_count,
        "max_word_count": input.max_word_count,
        "client_rag_embedding_identity": input.client_rag_embedding_identity,
        "client_rag_query_fingerprint": input.client_rag_query_fingerprint,
    })
}

fn inspiration_generation_input_fingerprint(input: &StartInspirationGenerationInput) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "inspiration_generation",
        "project_id": input.project_id,
        "message": input.message,
        "variables": null,
    })
}

fn outline_generation_input_fingerprint(input: &StartOutlineGenerationInput) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "outline_generation",
        "project_id": input.project_id,
        "operation": &input.operation,
    })
}

fn outline_generation_operation_kind(operation: &OutlineGenerationOperation) -> &'static str {
    match operation {
        OutlineGenerationOperation::GenerateParts { .. } => "generate_parts",
        OutlineGenerationOperation::RegenerateParts { .. } => "regenerate_parts",
        OutlineGenerationOperation::RegenerateLastPart { .. } => "regenerate_last_part",
        OutlineGenerationOperation::ContinuePartOutlines { .. } => "continue_part_outlines",
        OutlineGenerationOperation::RegeneratePartOutline { .. } => "regenerate_part_outline",
        OutlineGenerationOperation::GeneratePartChapterOutlines { .. } => {
            "generate_part_chapter_outlines"
        }
        OutlineGenerationOperation::BatchGeneratePartOutlines { .. } => {
            "batch_generate_part_outlines"
        }
        OutlineGenerationOperation::GenerateChapterOutlinesByCount { .. } => {
            "generate_chapter_outlines_by_count"
        }
        OutlineGenerationOperation::RegenerateChapterOutline { .. } => "regenerate_chapter_outline",
        OutlineGenerationOperation::ApplyBeatMapping { .. } => "apply_beat_mapping",
    }
}

fn outline_generation_target_id(project_id: i64, operation: &OutlineGenerationOperation) -> String {
    match operation {
        OutlineGenerationOperation::RegeneratePartOutline { part_id, .. }
        | OutlineGenerationOperation::GeneratePartChapterOutlines { part_id }
        | OutlineGenerationOperation::ApplyBeatMapping { part_id, .. } => {
            format!("{project_id}:part:{part_id}")
        }
        OutlineGenerationOperation::RegenerateChapterOutline { outline_id, .. } => {
            format!("{project_id}:outline:{outline_id}")
        }
        _ => project_id.to_string(),
    }
}

fn portrait_generation_input_fingerprint(input: &StartPortraitGenerationInput) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "portrait_generation",
        "project_id": input.project_id,
        "operation": &input.operation,
    })
}

fn portrait_generation_operation_kind(operation: &PortraitGenerationOperation) -> &'static str {
    match operation {
        PortraitGenerationOperation::Generate { .. } => "generate",
        PortraitGenerationOperation::Regenerate { .. } => "regenerate",
        PortraitGenerationOperation::GenerateAgeVariant { .. } => "age_variant",
        PortraitGenerationOperation::GenerateMissing { .. } => "generate_missing",
    }
}

fn portrait_generation_target_id(
    project_id: i64,
    operation: &PortraitGenerationOperation,
) -> String {
    match operation {
        PortraitGenerationOperation::Regenerate { portrait_id, .. }
        | PortraitGenerationOperation::GenerateAgeVariant { portrait_id, .. } => {
            format!("{project_id}:portrait:{portrait_id}")
        }
        _ => project_id.to_string(),
    }
}

fn image_generation_input_fingerprint(input: &StartImageGenerationInput) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "image_generation",
        "project_id": input.project_id,
        "operation": &input.operation,
    })
}

fn image_generation_operation_kind(operation: &ImageGenerationOperation) -> &'static str {
    match operation {
        ImageGenerationOperation::Scene { .. } => "scene",
        ImageGenerationOperation::Page { .. } => "page",
        ImageGenerationOperation::Batch { .. } => "batch",
        ImageGenerationOperation::ObjectProfile { .. } => "object_profile",
        ImageGenerationOperation::SceneProfile { .. } => "scene_profile",
    }
}

fn image_generation_target_id(project_id: i64, operation: &ImageGenerationOperation) -> String {
    match operation {
        ImageGenerationOperation::ObjectProfile {
            object_profile_id, ..
        } => format!("{project_id}:object_profile:{object_profile_id}"),
        ImageGenerationOperation::SceneProfile {
            scene_profile_id, ..
        } => format!("{project_id}:scene_profile:{scene_profile_id}"),
        _ => project_id.to_string(),
    }
}

fn video_generation_input_fingerprint(input: &StartVideoGenerationInput) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "video_generation",
        "project_id": input.project_id,
        "operation": &input.operation,
    })
}

fn video_generation_business_key(project_id: i64, operation_kind: &str, target_id: &str) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "video_generation",
        "project_id": project_id,
        "operation_kind": operation_kind,
        "target_id": target_id,
    })
}

fn video_generation_target_id(
    project_id: i64,
    operation: &VideoGenerationOperation,
) -> Result<String, TimelineError> {
    Ok(match operation {
        VideoGenerationOperation::Chapter {
            chapter_id,
            prompt_id,
            ..
        } => format!("{project_id}:chapter:{chapter_id}:prompt:{prompt_id}"),
        VideoGenerationOperation::ChapterBatch { chapter_id, .. } => {
            format!("{project_id}:chapter:{chapter_id}")
        }
        VideoGenerationOperation::ReferenceMedia { chapter_id, input } => {
            format!(
                "{project_id}:chapter:{chapter_id}:reference_media:plan:{}:shot:{}:index:{}",
                input.segment_plan_prompt_id,
                input.target_segment.source_shot_id,
                input.target_segment.segment_index_in_shot,
            )
        }
        VideoGenerationOperation::PromptSegmentPlan { chapter_id, .. } => {
            format!("{project_id}:chapter:{chapter_id}:prompt_segment_plan:shared")
        }
        VideoGenerationOperation::PromptInsertedSegment {
            chapter_id,
            options,
        } => {
            let gap_key = VideoTimelineGap {
                previous_item_id: options.previous_item_id,
                next_item_id: options.next_item_id,
            }
            .canonical_key()?;
            format!(
                "{project_id}:chapter:{chapter_id}:prompt_inserted_segment:timeline:{}:gap:{gap_key}",
                options.timeline_id,
            )
        }
        VideoGenerationOperation::PromptFrames {
            chapter_id,
            options,
        }
        | VideoGenerationOperation::PromptVideos {
            chapter_id,
            options,
        } => video_prompt_unit_target_id(project_id, *chapter_id, operation.kind(), options),
    })
}

fn video_prompt_unit_target_id(
    project_id: i64,
    chapter_id: i64,
    operation_kind: &str,
    options: &VideoPromptGenerationOptions,
) -> String {
    let segment_plan_prompt_id = options
        .segment_plan_prompt_id
        .expect("video prompt start validation requires segment plan");
    let target_segment = options
        .target_segment
        .as_ref()
        .expect("video prompt start validation requires target segment");
    format!(
        "{project_id}:chapter:{chapter_id}:{operation_kind}:plan:{segment_plan_prompt_id}:shot:{}:index:{}",
        target_segment.source_shot_id, target_segment.segment_index_in_shot,
    )
}

fn validate_video_prompt_generation_start(
    chapter_id: i64,
    options: &VideoPromptGenerationOptions,
    validate_options: fn(&VideoPromptGenerationOptions) -> Result<(), AppError>,
) -> Result<(), AppError> {
    if chapter_id <= 0 {
        return Err(AppError::InvalidArgument(
            "视频提示词生成 chapter_id 必须为正数".to_string(),
        ));
    }
    validate_options(options)
}

fn blueprint_generation_input_fingerprint(input: &StartBlueprintGenerationInput) -> Value {
    let variables = input
        .variables
        .as_ref()
        .map(|value| serde_json::to_value(value).expect("BTreeMap values serialize"))
        .unwrap_or_else(|| serde_json::json!({}));
    canonical_blueprint_generation_input_fingerprint(
        input.project_id,
        &input.title,
        &input.genre,
        input.target_length,
        &variables,
    )
}

fn blueprint_refinement_input_fingerprint(input: &StartBlueprintRefinementInput) -> Value {
    let variables = input
        .variables
        .as_ref()
        .map(|value| serde_json::to_value(value).expect("BTreeMap values serialize"))
        .unwrap_or_else(|| serde_json::json!({}));
    canonical_blueprint_refinement_input_fingerprint(
        input.project_id,
        input.focus.as_deref(),
        &variables,
    )
}

fn optimize_chat_generation_input_fingerprint(input: &StartOptimizeChatGenerationInput) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "optimization_chat",
        "session_id": input.request.session_id,
        "message": input.request.message,
        "permissions": input.request.permissions,
    })
}

fn optimize_analysis_generation_input_fingerprint(
    input: &StartOptimizeAnalysisGenerationInput,
) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "optimization_analysis",
        "session_id": input.session_id,
    })
}

fn optimize_patch_generation_input_fingerprint(input: &StartOptimizePatchGenerationInput) -> Value {
    serde_json::json!({
        "version": 1,
        "task_kind": "optimization_patch",
        "session_id": input.request.session_id,
        "issue_ids": input.request.issue_ids,
        "agent_instruction": input.request.agent_instruction,
    })
}

fn map_optimize_generation_start_error(error: AppError) -> GenerationTaskAccessError {
    match error {
        AppError::Forbidden(_) | AppError::Unauthorized(_) | AppError::NotFound(_) => {
            GenerationTaskAccessError::Project(error)
        }
        other => GenerationTaskAccessError::InvalidArgument(other.to_string()),
    }
}

fn map_outline_generation_start_error(error: AppError) -> GenerationTaskAccessError {
    match error {
        AppError::Forbidden(_) | AppError::Unauthorized(_) | AppError::NotFound(_) => {
            GenerationTaskAccessError::Project(error)
        }
        other => GenerationTaskAccessError::InvalidArgument(other.to_string()),
    }
}

fn map_portrait_generation_start_error(error: AppError) -> GenerationTaskAccessError {
    match error {
        AppError::Forbidden(_) | AppError::Unauthorized(_) | AppError::NotFound(_) => {
            GenerationTaskAccessError::Project(error)
        }
        other => GenerationTaskAccessError::InvalidArgument(other.to_string()),
    }
}

fn map_image_generation_start_error(error: AppError) -> GenerationTaskAccessError {
    match error {
        AppError::Forbidden(_) | AppError::Unauthorized(_) | AppError::NotFound(_) => {
            GenerationTaskAccessError::Project(error)
        }
        other => GenerationTaskAccessError::InvalidArgument(other.to_string()),
    }
}

fn map_video_generation_start_error(error: AppError) -> GenerationTaskAccessError {
    match error {
        AppError::Forbidden(_) | AppError::Unauthorized(_) | AppError::NotFound(_) => {
            GenerationTaskAccessError::Project(error)
        }
        other => GenerationTaskAccessError::InvalidArgument(other.to_string()),
    }
}

fn required_public_snapshot(
    task: a4n_db::generation::PersistedGenerationTask,
) -> Result<GenerationTaskSnapshot, AppError> {
    task.public_snapshot.ok_or_else(|| AppError::InvalidState {
        current: format!(
            "generation task {} 缺少 public snapshot",
            task.checkpoint.task_id
        ),
        operation: "初始化 generation session".to_string(),
    })
}

#[cfg(test)]
mod video_prompt_generation_tests {
    use super::*;
    use a4n_api_contract::{
        generation_tasks::GenerationStartRequestMeta,
        video::{ReferenceMediaVideoInput, VideoPromptGenerationOptions, VideoSegmentTarget},
    };

    fn options() -> VideoPromptGenerationOptions {
        VideoPromptGenerationOptions {
            workflow_mode: Some("storyboard".to_string()),
            segment_plan_prompt_id: Some(11),
            style: "电影感".to_string(),
            prompt_output_language: "zh_cn".to_string(),
            prompt_batch_size: 2,
            target_segment: Some(VideoSegmentTarget {
                source_shot_id: "shot-4".to_string(),
                segment_index_in_shot: 1,
            }),
            target_narrative_unit_index: None,
            duration_secs: 6,
            duration_secs_options: vec![6],
            aspect_ratio: "16:9".to_string(),
            reference_image_ids: vec![1],
            video_library_image_ids: vec![2],
            regenerate_existing: false,
        }
    }

    fn input(options: VideoPromptGenerationOptions) -> StartVideoGenerationInput {
        StartVideoGenerationInput {
            meta: GenerationStartRequestMeta {
                start_request_id: "request-1".to_string(),
                retry_of_task_id: None,
            },
            project_id: 19,
            operation: VideoGenerationOperation::PromptFrames {
                chapter_id: 3,
                options,
            },
        }
    }

    fn validate_start(options: &VideoPromptGenerationOptions) -> Result<(), AppError> {
        validate_video_prompt_generation_start(
            3,
            options,
            crate::video::validate_video_frame_prompt_generation_options,
        )
    }

    #[test]
    fn prompt_target_contains_chapter_operation_and_workflow() {
        let operation = input(options()).operation;
        assert_eq!(
            video_generation_target_id(19, &operation).unwrap(),
            "19:chapter:3:prompt_frames:plan:11:shot:shot-4:index:1"
        );
    }

    #[test]
    fn prompt_business_key_uses_canonical_target() {
        let input = input(options());
        let target_id = video_generation_target_id(input.project_id, &input.operation).unwrap();
        assert_eq!(
            video_generation_business_key(input.project_id, input.operation.kind(), &target_id,),
            serde_json::json!({
                "version": 1,
                "task_kind": "video_generation",
                "project_id": 19,
                "operation_kind": "prompt_frames",
                "target_id": "19:chapter:3:prompt_frames:plan:11:shot:shot-4:index:1",
            })
        );
    }

    #[test]
    fn reference_media_target_identifies_plan_and_segment() {
        let operation = VideoGenerationOperation::ReferenceMedia {
            chapter_id: 3,
            input: ReferenceMediaVideoInput {
                segment_plan_prompt_id: 11,
                target_segment: VideoSegmentTarget {
                    source_shot_id: "shot-4".to_string(),
                    segment_index_in_shot: 1,
                },
                image_uploads: vec![],
                video_uploads: vec![],
                audio_uploads: vec![],
                config_id: None,
            },
        };
        assert_eq!(
            video_generation_target_id(19, &operation).unwrap(),
            "19:chapter:3:reference_media:plan:11:shot:shot-4:index:1"
        );
    }

    #[test]
    fn prompt_fingerprint_changes_with_options() {
        let first = input(options());
        let mut changed_options = options();
        changed_options.style = "水彩".to_string();
        let second = input(changed_options);
        assert_ne!(
            video_generation_input_fingerprint(&first),
            video_generation_input_fingerprint(&second)
        );
    }

    #[test]
    fn prompt_start_validation_rejects_missing_contract_fields() {
        let mut invalid = options();
        invalid.workflow_mode = Some(" ".to_string());
        assert!(validate_start(&invalid).is_err());

        invalid = options();
        invalid.style.clear();
        assert!(validate_start(&invalid).is_err());

        invalid = options();
        invalid.prompt_batch_size = 0;
        assert!(validate_start(&invalid).is_err());

        invalid = options();
        invalid.duration_secs = 0;
        assert!(validate_start(&invalid).is_err());
    }
}
