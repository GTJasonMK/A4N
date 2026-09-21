//! Runtime-facing generation task contracts.
//!
//! Queue rows, checkpoints and live session state stay in the generation
//! owner. This crate exposes only stable summaries, replay snapshots and
//! lifecycle inputs required by Runtime and Host adapters.

use a4n_api_contract::generation_tasks::{GenerationTaskHandle, GenerationTaskSummary};
use a4n_domain::generation::{GenerationTaskListCursor, ProjectLifecycleOperation};
use a4n_error::AppError;
use a4n_service_protocol::{GenerationTaskEvent, GenerationTaskSnapshot, InterruptionReason};
use async_trait::async_trait;
use std::error::Error;
use tokio::sync::watch;

#[derive(Debug, Clone)]
pub struct GenerationTaskAccessError {
    code: String,
    message: String,
    retryable: bool,
}

impl GenerationTaskAccessError {
    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn retryable(&self) -> bool {
        self.retryable
    }
}

impl std::fmt::Display for GenerationTaskAccessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for GenerationTaskAccessError {}

#[derive(Debug, Clone)]
pub struct GenerationTaskHistorySummaries {
    pub items: Vec<GenerationTaskSummary>,
    pub next_cursor: Option<GenerationTaskListCursor>,
}

#[derive(Debug, Clone)]
pub enum GenerationReplay {
    Events {
        events: Vec<GenerationTaskEvent>,
        latest_sequence: u64,
        replay_floor: u64,
    },
    SnapshotRequired {
        snapshot: GenerationTaskSnapshot,
        latest_sequence: u64,
        replay_floor: u64,
    },
}

pub trait GenerationSubscriptionPort: Send {
    fn take_initial(&mut self) -> Option<GenerationReplay>;

    fn receiver(&mut self) -> &mut watch::Receiver<u64>;

    fn replay_after(&self, after_sequence: Option<u64>) -> GenerationReplay;

    fn snapshot(&self) -> GenerationTaskSnapshot;
}

pub struct GenerationSubscription {
    inner: Box<dyn GenerationSubscriptionPort>,
}

impl GenerationSubscription {
    pub fn new(inner: Box<dyn GenerationSubscriptionPort>) -> Self {
        Self { inner }
    }

    pub fn take_initial(&mut self) -> Option<GenerationReplay> {
        self.inner.take_initial()
    }

    pub fn receiver(&mut self) -> &mut watch::Receiver<u64> {
        self.inner.receiver()
    }

    pub fn replay_after(&self, after_sequence: Option<u64>) -> GenerationReplay {
        self.inner.replay_after(after_sequence)
    }

    pub fn snapshot(&self) -> GenerationTaskSnapshot {
        self.inner.snapshot()
    }
}

pub trait AuthorizedGenerationTaskPort: Send + Sync {
    fn public_snapshot(&self) -> Option<&GenerationTaskSnapshot>;

    fn snapshot_expired(&self) -> bool;

    fn covers_through_sequence(&self) -> i64;

    fn subscribe(
        &self,
        after_sequence: Option<u64>,
    ) -> Result<GenerationSubscription, GenerationTaskAccessError>;
}

pub struct AuthorizedGenerationTask {
    inner: Box<dyn AuthorizedGenerationTaskPort>,
}

impl AuthorizedGenerationTask {
    pub fn new(inner: Box<dyn AuthorizedGenerationTaskPort>) -> Self {
        Self { inner }
    }

    pub fn public_snapshot(&self) -> Option<&GenerationTaskSnapshot> {
        self.inner.public_snapshot()
    }

    pub fn snapshot_expired(&self) -> bool {
        self.inner.snapshot_expired()
    }

    pub fn covers_through_sequence(&self) -> i64 {
        self.inner.covers_through_sequence()
    }

    pub fn subscribe(
        &self,
        after_sequence: Option<u64>,
    ) -> Result<GenerationSubscription, GenerationTaskAccessError> {
        self.inner.subscribe(after_sequence)
    }
}

#[async_trait]
pub trait GenerationTaskReadApplicationPort: Send + Sync {
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
}

#[derive(Debug, Clone)]
pub struct GenerationTerminalError {
    code: String,
    message: String,
    retryable: bool,
}

impl GenerationTerminalError {
    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn retryable(&self) -> bool {
        self.retryable
    }
}

impl std::fmt::Display for GenerationTerminalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for GenerationTerminalError {}

#[async_trait]
pub trait GenerationTaskMutationApplicationPort: Send + Sync {
    async fn request_generation_cancel(
        &self,
        requested_by_user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<GenerationTaskHandle, GenerationTerminalError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationTerminalWriteOutcome {
    Persisted,
    RebuiltFromPersistedSnapshot,
    AlreadyTerminal,
    NoLiveSession,
}

#[derive(Debug, Clone)]
pub struct GenerationLifecycleError {
    code: String,
    message: String,
    retryable: bool,
}

impl GenerationLifecycleError {
    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn retryable(&self) -> bool {
        self.retryable
    }
}

impl std::fmt::Display for GenerationLifecycleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for GenerationLifecycleError {}

#[derive(Debug, Clone)]
pub enum ProjectGeneratedDataResetApplicationResult {
    Standard(a4n_domain::project_reset::ProjectGeneratedDataResetOutcome),
    Freeform {
        result: a4n_domain::project_reset::FreeformVideoGeneratedDataResetResult,
    },
}

#[async_trait]
pub trait GenerationLifecycleApplicationPort: Send + Sync {
    async fn begin_generation_project_lifecycle_gate(
        &self,
        project_id: i64,
        user_id: i64,
        operation: ProjectLifecycleOperation,
    ) -> Result<a4n_domain::generation::ProjectLifecycleGateClaim, GenerationLifecycleError>;

    async fn begin_generation_user_disable_gate(
        &self,
        caller_user_id: i64,
        user_id: i64,
    ) -> Result<a4n_domain::generation::UserLifecycleGateClaim, GenerationLifecycleError>;

    async fn active_generation_tasks_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_domain::generation::GenerationLifecycleTask>, GenerationLifecycleError>;

    async fn active_generation_tasks_for_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<a4n_domain::generation::GenerationLifecycleTask>, GenerationLifecycleError>;

    async fn active_generation_tasks(
        &self,
    ) -> Result<Vec<a4n_domain::generation::GenerationLifecycleTask>, GenerationLifecycleError>;

    async fn pending_generation_lifecycle_operations(
        &self,
    ) -> Result<Vec<a4n_domain::generation::PendingLifecycleOperation>, GenerationLifecycleError>;

    async fn request_generation_destructive_cancel(
        &self,
        task: &a4n_domain::generation::GenerationLifecycleTask,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError>;

    async fn interrupt_generation_destructive_task(
        &self,
        task: &a4n_domain::generation::GenerationLifecycleTask,
        reason: InterruptionReason,
        summary: String,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError>;

    async fn restore_generation_project_lifecycle_gate(
        &self,
        project_id: i64,
        operation: ProjectLifecycleOperation,
    ) -> Result<(), GenerationLifecycleError>;

    async fn restore_generation_user_lifecycle_gate(
        &self,
        user_id: i64,
    ) -> Result<(), GenerationLifecycleError>;

    async fn complete_generation_project_reset_gate(
        &self,
        project_id: i64,
    ) -> Result<(), GenerationLifecycleError>;

    async fn complete_generation_user_disable_gate(
        &self,
        user_id: i64,
    ) -> Result<(), GenerationLifecycleError>;

    async fn delete_project_cascade_validated(&self, project_id: i64) -> Result<(), AppError>;

    async fn reset_project_generated_data_validated(
        &self,
        project_id: i64,
    ) -> Result<ProjectGeneratedDataResetApplicationResult, AppError>;
}

#[async_trait]
pub trait GenerationCleanupApplicationPort: Send + Sync {
    async fn expire_generation_snapshot_bodies(
        &self,
        limit: usize,
    ) -> Result<u64, GenerationLifecycleError>;

    async fn purge_expired_generation_task_trees(
        &self,
        limit: usize,
    ) -> Result<u64, GenerationLifecycleError>;
}
