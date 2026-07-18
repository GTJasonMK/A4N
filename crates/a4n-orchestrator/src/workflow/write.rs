use super::{
    CancelWorkflowRunPersistenceResult, CompletedLlmCallCheckpointInput,
    UpsertWorkflowCheckpointInput, UpsertWorkflowRunInput, WorkflowProgressEventInput,
    workflow_write_app_error,
};
use crate::OrchestratorContext;
use a4n_core::error::AppError;
use a4n_core::models::generation::{workflow_checkpoint, workflow_progress_event, workflow_run};

impl OrchestratorContext {
    pub async fn cancel_workflow_run_persistence(
        &self,
        workflow_run_id: i64,
        reason: &str,
    ) -> Result<CancelWorkflowRunPersistenceResult, AppError> {
        a4n_db::workflow::cancel_workflow_run_persistence(
            self.db.connection(),
            workflow_run_id,
            reason,
        )
        .await
        .map_err(workflow_write_app_error)
    }

    pub async fn upsert_workflow_run(
        &self,
        input: UpsertWorkflowRunInput,
    ) -> Result<workflow_run::Model, AppError> {
        a4n_db::workflow::upsert_workflow_run(self.db.connection(), input)
            .await
            .map_err(workflow_write_app_error)
    }

    pub async fn mark_workflow_status(
        &self,
        workflow_run_id: i64,
        status: &str,
        can_resume: bool,
        error_message: Option<&str>,
    ) -> Result<workflow_run::Model, AppError> {
        a4n_db::workflow::mark_workflow_status(
            self.db.connection(),
            workflow_run_id,
            status,
            can_resume,
            error_message,
        )
        .await
        .map_err(workflow_write_app_error)
    }

    pub async fn mark_workflow_current_step(
        &self,
        workflow_run_id: i64,
        current_step_key: &str,
    ) -> Result<workflow_run::Model, AppError> {
        a4n_db::workflow::mark_workflow_current_step(
            self.db.connection(),
            workflow_run_id,
            current_step_key,
        )
        .await
        .map_err(workflow_write_app_error)
    }

    pub async fn upsert_workflow_checkpoint(
        &self,
        input: UpsertWorkflowCheckpointInput,
    ) -> Result<workflow_checkpoint::Model, AppError> {
        a4n_db::workflow::upsert_workflow_checkpoint(self.db.connection(), input)
            .await
            .map_err(workflow_write_app_error)
    }

    pub async fn record_workflow_progress_event(
        &self,
        input: WorkflowProgressEventInput<'_>,
    ) -> Result<workflow_progress_event::Model, AppError> {
        a4n_db::workflow::record_workflow_progress_event(self.db.connection(), input)
            .await
            .map_err(workflow_write_app_error)
    }

    /// 保存已完成的 LLM call checkpoint。
    pub async fn save_completed_llm_call_checkpoint(
        &self,
        input: CompletedLlmCallCheckpointInput,
    ) -> Result<(), AppError> {
        a4n_db::workflow::save_completed_llm_call_checkpoint(self.db.connection(), input)
            .await
            .map_err(workflow_write_app_error)
    }
}
