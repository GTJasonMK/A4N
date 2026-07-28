use a4n_api_contract::generation_tasks::{GenerationTaskHandle, GenerationTaskSummary};
use a4n_db::generation::{
    GenerationLifecycleGateError, GenerationLifecycleTask, GenerationTaskLifecycleError,
    GenerationTaskListCursor, PendingLifecycleOperation, PersistedGenerationTask,
    ProjectLifecycleGateClaim, ProjectLifecycleOperation, UserLifecycleGateClaim,
};
use a4n_orchestrator::{
    GenerationTaskAccessError, GenerationTaskHistorySummaries, OrchestratorContext,
    generation::events::{GenerationReplay, GenerationSubscription},
    generation::terminal::{GenerationTerminalError, GenerationTerminalWriteOutcome},
};
use a4n_service_protocol::InterruptionReason;

pub(crate) struct GenerationTasksRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> GenerationTasksRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn authorize_task(
        &self,
        user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<PersistedGenerationTask, GenerationTaskAccessError> {
        self.orchestrator
            .authorize_generation_task(user_id, task_id, attempt)
            .await
    }

    pub(crate) async fn task_summary(
        &self,
        user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<GenerationTaskSummary, GenerationTaskAccessError> {
        self.orchestrator
            .generation_task_summary_for_user(user_id, task_id, attempt)
            .await
    }

    pub(crate) async fn list_active_summaries(
        &self,
        user_id: i64,
        project_id: Option<i64>,
    ) -> Result<Vec<GenerationTaskSummary>, GenerationTaskAccessError> {
        self.orchestrator
            .list_active_generation_task_summaries(user_id, project_id)
            .await
    }

    pub(crate) async fn list_history_summaries(
        &self,
        user_id: i64,
        project_id: Option<i64>,
        limit: u32,
        cursor: Option<GenerationTaskListCursor>,
    ) -> Result<GenerationTaskHistorySummaries, GenerationTaskAccessError> {
        self.orchestrator
            .list_generation_task_history_summaries(user_id, project_id, limit, cursor)
            .await
    }

    pub(crate) fn subscribe_task(
        &self,
        task: &PersistedGenerationTask,
        after_sequence: Option<u64>,
    ) -> Result<GenerationSubscription, GenerationTaskAccessError> {
        self.orchestrator
            .subscribe_generation_task(task, after_sequence)
    }

    pub(crate) fn replay_task(
        &self,
        task_id: &str,
        attempt: u32,
        after_sequence: Option<u64>,
    ) -> Result<GenerationReplay, GenerationTaskAccessError> {
        self.orchestrator
            .generation_task_replay(task_id, attempt, after_sequence)
    }

    pub(crate) async fn begin_project_gate(
        &self,
        project_id: i64,
        user_id: i64,
        operation: ProjectLifecycleOperation,
    ) -> Result<ProjectLifecycleGateClaim, GenerationLifecycleGateError> {
        self.orchestrator
            .begin_generation_project_lifecycle_gate(project_id, user_id, operation)
            .await
    }

    pub(crate) async fn begin_user_disable_gate(
        &self,
        caller_user_id: i64,
        user_id: i64,
    ) -> Result<UserLifecycleGateClaim, GenerationLifecycleGateError> {
        self.orchestrator
            .begin_generation_user_disable_gate(caller_user_id, user_id)
            .await
    }

    pub(crate) async fn active_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError> {
        self.orchestrator
            .active_generation_tasks_for_project(project_id)
            .await
    }

    pub(crate) async fn active_for_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError> {
        self.orchestrator
            .active_generation_tasks_for_user(user_id)
            .await
    }

    pub(crate) async fn active(
        &self,
    ) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError> {
        self.orchestrator.active_generation_tasks().await
    }

    pub(crate) async fn expire_snapshot_bodies(
        &self,
        limit: usize,
    ) -> Result<u64, GenerationTaskLifecycleError> {
        self.orchestrator
            .expire_generation_snapshot_bodies(limit)
            .await
    }

    pub(crate) async fn purge_expired_task_trees(
        &self,
        limit: usize,
    ) -> Result<u64, GenerationTaskLifecycleError> {
        self.orchestrator
            .purge_expired_generation_task_trees(limit)
            .await
    }

    pub(crate) async fn pending_operations(
        &self,
    ) -> Result<Vec<PendingLifecycleOperation>, GenerationLifecycleGateError> {
        self.orchestrator
            .pending_generation_lifecycle_operations()
            .await
    }

    pub(crate) async fn request_destructive_cancel(
        &self,
        task: &GenerationLifecycleTask,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        self.orchestrator
            .request_generation_destructive_cancel(task)
            .await
    }

    pub(crate) async fn request_cancel(
        &self,
        requested_by_user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<GenerationTaskHandle, GenerationTerminalError> {
        self.orchestrator
            .request_generation_cancel(requested_by_user_id, task_id, attempt)
            .await
    }

    pub(crate) async fn interrupt(
        &self,
        task: &GenerationLifecycleTask,
        reason: InterruptionReason,
        summary: String,
    ) -> Result<GenerationTerminalWriteOutcome, GenerationTerminalError> {
        self.orchestrator
            .interrupt_generation_destructive_task(task, reason, summary)
            .await
    }

    pub(crate) async fn restore_project_gate(
        &self,
        project_id: i64,
        operation: ProjectLifecycleOperation,
    ) -> Result<(), GenerationLifecycleGateError> {
        self.orchestrator
            .restore_generation_project_lifecycle_gate(project_id, operation)
            .await
    }

    pub(crate) async fn restore_user_gate(
        &self,
        user_id: i64,
    ) -> Result<(), GenerationLifecycleGateError> {
        self.orchestrator
            .restore_generation_user_lifecycle_gate(user_id)
            .await
    }

    pub(crate) async fn complete_project_reset(
        &self,
        project_id: i64,
    ) -> Result<(), GenerationLifecycleGateError> {
        self.orchestrator
            .complete_generation_project_reset_gate(project_id)
            .await
    }

    pub(crate) async fn complete_user_disable(
        &self,
        user_id: i64,
    ) -> Result<(), GenerationLifecycleGateError> {
        self.orchestrator
            .complete_generation_user_disable_gate(user_id)
            .await
    }
}
