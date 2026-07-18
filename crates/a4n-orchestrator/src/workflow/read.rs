use super::{
    ListWorkflowRunsInput, PersistedTaskInfo, TaskProgressEventSummary, WorkflowRunDetailSummary,
    WorkflowRunSummary, workflow_read_app_error,
};
use crate::OrchestratorContext;
use a4n_core::error::AppError;
use a4n_core::models::generation::{workflow_checkpoint, workflow_run};

impl OrchestratorContext {
    pub async fn workflow_runs(
        &self,
        input: ListWorkflowRunsInput,
    ) -> Result<Vec<WorkflowRunSummary>, AppError> {
        a4n_db::workflow::list_workflow_runs(self.db.connection(), input)
            .await
            .map_err(workflow_read_app_error)
    }

    pub async fn workflow_run_summary(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowRunSummary, AppError> {
        a4n_db::workflow::workflow_run_summary_by_id(self.db.connection(), workflow_run_id)
            .await
            .map_err(workflow_read_app_error)
    }

    pub async fn workflow_run_detail(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowRunDetailSummary, AppError> {
        a4n_db::workflow::workflow_run_detail(self.db.connection(), workflow_run_id)
            .await
            .map_err(workflow_read_app_error)
    }

    pub async fn workflow_progress_events(
        &self,
        workflow_run_id: i64,
        limit: usize,
    ) -> Result<Vec<TaskProgressEventSummary>, AppError> {
        a4n_db::workflow::workflow_progress_events(self.db.connection(), workflow_run_id, limit)
            .await
            .map_err(workflow_read_app_error)
    }

    pub async fn persisted_task_history(
        &self,
        project_id: Option<i64>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<PersistedTaskInfo>, AppError> {
        a4n_db::workflow::persisted_task_history(self.db.connection(), project_id, offset, limit)
            .await
            .map_err(workflow_read_app_error)
    }

    pub async fn persisted_task_by_id(
        &self,
        task_id: &str,
    ) -> Result<Option<PersistedTaskInfo>, AppError> {
        a4n_db::workflow::persisted_task_by_id(self.db.connection(), task_id)
            .await
            .map_err(workflow_read_app_error)
    }

    pub async fn workflow_run_models(
        &self,
        input: ListWorkflowRunsInput,
    ) -> Result<Vec<workflow_run::Model>, AppError> {
        a4n_db::workflow::list_workflow_run_models(self.db.connection(), input)
            .await
            .map_err(workflow_read_app_error)
    }

    pub async fn workflow_runs_by_project_kind(
        &self,
        project_id: i64,
        workflow_kind: &str,
    ) -> Result<Vec<workflow_run::Model>, AppError> {
        a4n_db::workflow::workflow_runs_by_project_kind(
            self.db.connection(),
            project_id,
            workflow_kind,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    pub async fn workflow_run_model_by_id(
        &self,
        workflow_run_id: i64,
    ) -> Result<workflow_run::Model, AppError> {
        a4n_db::workflow::workflow_run_model_by_id(self.db.connection(), workflow_run_id)
            .await
            .map_err(workflow_read_app_error)
    }

    pub async fn workflow_run_model_optional_by_id(
        &self,
        workflow_run_id: i64,
    ) -> Result<Option<workflow_run::Model>, AppError> {
        a4n_db::workflow::workflow_run_model_optional_by_id(self.db.connection(), workflow_run_id)
            .await
            .map_err(workflow_read_app_error)
    }

    pub async fn latest_workflow_run_by_business_key(
        &self,
        business_key: &str,
    ) -> Result<Option<workflow_run::Model>, AppError> {
        a4n_db::workflow::latest_workflow_run_by_business_key(self.db.connection(), business_key)
            .await
            .map_err(workflow_read_app_error)
    }

    pub async fn workflow_checkpoint_models(
        &self,
        workflow_run_id: i64,
    ) -> Result<Vec<workflow_checkpoint::Model>, AppError> {
        a4n_db::workflow::workflow_checkpoint_models(self.db.connection(), workflow_run_id)
            .await
            .map_err(workflow_read_app_error)
    }

    pub async fn completed_workflow_checkpoint_by_fingerprint(
        &self,
        workflow_run_id: i64,
        step_key: &str,
        input_fingerprint_sha256: &str,
    ) -> Result<Option<workflow_checkpoint::Model>, AppError> {
        a4n_db::workflow::completed_workflow_checkpoint_by_fingerprint(
            self.db.connection(),
            workflow_run_id,
            step_key,
            input_fingerprint_sha256,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    /// 加载已完成的 LLM call checkpoint（含 fingerprint 计算与合同校验入口）。
    pub async fn load_completed_llm_call_checkpoint(
        &self,
        workflow_run_id: i64,
        call_key: &str,
        input_fingerprint_json: &serde_json::Value,
    ) -> Result<Option<workflow_checkpoint::Model>, AppError> {
        a4n_db::workflow::load_completed_llm_call_checkpoint(
            self.db.connection(),
            workflow_run_id,
            call_key,
            input_fingerprint_json,
        )
        .await
        .map_err(workflow_read_app_error)
    }
}
