//! Workflow 领域 runtime 窄 facade。

use a4n_core::error::AppError;
use a4n_core::models::generation::workflow_run;
use a4n_orchestrator::{
    OrchestratorContext,
    workflow::{
        CancelWorkflowRunPersistenceResult, GetWorkflowRunInput, ListWorkflowRunsInput,
        PersistedTaskInfo, TaskProgressEventSummary, WorkflowRunDetailSummary, WorkflowRunSummary,
    },
};

pub(crate) struct WorkflowRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> WorkflowRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn workflow_runs(
        &self,
        input: ListWorkflowRunsInput,
    ) -> Result<Vec<WorkflowRunSummary>, AppError> {
        self.orchestrator.workflow_runs(input).await
    }

    pub(crate) async fn workflow_run_summary(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowRunSummary, AppError> {
        self.orchestrator
            .workflow_run_summary(workflow_run_id)
            .await
    }

    pub(crate) async fn workflow_run_detail(
        &self,
        input: GetWorkflowRunInput,
    ) -> Result<WorkflowRunDetailSummary, AppError> {
        self.orchestrator.workflow_run_detail(input).await
    }

    pub(crate) async fn workflow_progress_events(
        &self,
        workflow_run_id: i64,
        limit: usize,
    ) -> Result<Vec<TaskProgressEventSummary>, AppError> {
        self.orchestrator
            .workflow_progress_events(workflow_run_id, limit)
            .await
    }

    pub(crate) async fn persisted_task_history(
        &self,
        project_id: Option<i64>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<PersistedTaskInfo>, AppError> {
        self.orchestrator
            .persisted_task_history(project_id, offset, limit)
            .await
    }

    pub(crate) async fn persisted_task_by_id(
        &self,
        task_id: &str,
    ) -> Result<Option<PersistedTaskInfo>, AppError> {
        self.orchestrator.persisted_task_by_id(task_id).await
    }

    pub(crate) async fn workflow_run_model_by_active_task_id(
        &self,
        task_id: &str,
    ) -> Result<Option<workflow_run::Model>, AppError> {
        self.orchestrator
            .workflow_run_model_by_active_task_id(task_id)
            .await
    }

    pub(crate) async fn workflow_runs_by_project_kind(
        &self,
        project_id: i64,
        workflow_kind: &str,
    ) -> Result<Vec<workflow_run::Model>, AppError> {
        self.orchestrator
            .workflow_runs_by_project_kind(project_id, workflow_kind)
            .await
    }

    pub(crate) async fn workflow_run_model_by_id(
        &self,
        workflow_run_id: i64,
    ) -> Result<workflow_run::Model, AppError> {
        self.orchestrator
            .workflow_run_model_by_id(workflow_run_id)
            .await
    }

    pub(crate) async fn mark_workflow_running(
        &self,
        workflow_run_id: i64,
    ) -> Result<workflow_run::Model, AppError> {
        self.orchestrator
            .mark_workflow_running_validated(workflow_run_id)
            .await
    }

    /// 取消 workflow 持久化并中断关联生成视频（访问校验与进程内取消由调用方完成）。
    pub(crate) async fn cancel_workflow_run_and_interrupt_videos(
        &self,
        workflow_run_id: i64,
        reason: &str,
    ) -> Result<CancelWorkflowRunPersistenceResult, AppError> {
        self.orchestrator
            .cancel_workflow_run_and_interrupt_videos(workflow_run_id, reason)
            .await
    }
}
