//! 生成运行账本领域私有 owner。
//!
//! 持有 `DbHandle`，封装 generation_run / llm_call_trace 的落库与按目标完成/失败。

use a4n_api_contract::{
    blueprint::{StartBlueprintGenerationInput, StartBlueprintRefinementInput},
    chapter::StartChapterGenerationInput,
    generation_tasks::{
        GenerationSnapshotState, GenerationStartRequestMeta, GenerationTaskHandle,
        GenerationTaskStartError, GenerationTaskSummary,
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
use a4n_generation_host_contract::{
    AuthorizedGenerationTask as HostAuthorizedGenerationTask,
    AuthorizedGenerationTaskPort as HostAuthorizedGenerationTaskPort,
    GenerationCleanupApplicationPort as HostGenerationCleanupApplicationPort,
    GenerationLifecycleApplicationPort as HostGenerationLifecycleApplicationPort,
    GenerationLifecycleError as HostGenerationLifecycleError,
    GenerationReplay as HostGenerationReplay, GenerationSubscription as HostGenerationSubscription,
    GenerationSubscriptionPort as HostGenerationSubscriptionPort,
    GenerationTaskAccessError as HostGenerationTaskAccessError,
    GenerationTaskHistorySummaries as HostGenerationTaskHistorySummaries,
    GenerationTaskMutationApplicationPort as HostGenerationTaskMutationApplicationPort,
    GenerationTaskReadApplicationPort as HostGenerationTaskReadApplicationPort,
    GenerationTerminalError as HostGenerationTerminalError,
    GenerationTerminalWriteOutcome as HostGenerationTerminalWriteOutcome,
    ProjectGeneratedDataResetApplicationResult as HostResetApplicationResult,
};
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
        GenerationReplay as OwnerGenerationReplay, GenerationSessionConfig, GenerationSessionError,
        GenerationSessionRegistry, GenerationSubscription as OwnerGenerationSubscription,
        GenerationTaskSession, SessionGenerationEventPublisher,
    },
    generation_error, generation_failure_is_cancelled_or_interrupted,
    terminal::{
        CompletionDescriptor, GenerationTerminalCoordinator, GenerationTerminalError,
        GenerationTerminalWriteOutcome,
    },
};
use crate::video::validate_reference_media_start_input;
pub(crate) use a4n_api_contract::generation_tasks::GenerationTaskStart;
use a4n_task_runtime::persistent::PersistentTaskQueue;

/// 生成运行账本 owner；章节生成服务可持有克隆以写 generation run。
#[derive(Clone)]
pub(crate) struct GenerationApplication {
    database: DbHandle,
    task_queue: Arc<PersistentTaskQueue>,
    terminal: GenerationTerminalCoordinator,
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

    pub(crate) fn into_start_error(self) -> GenerationTaskStartError {
        let code = self.code();
        let retryable = !matches!(
            code,
            "generation_request_invalid"
                | "generation_task_forbidden"
                | "generation_task_unauthenticated"
                | "generation_task_not_found"
        );
        GenerationTaskStartError::new(code, self.to_string(), retryable)
    }
}

mod adapters;
mod execution;
mod fingerprints;
mod lifecycle;
mod read_subscribe;
mod reservation;
mod start;

pub(crate) use execution::{GenerationTaskExecution, fail_generation_workflow_execution};
