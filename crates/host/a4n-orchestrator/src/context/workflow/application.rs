//! workflow 数据库 owner。

use super::*;

impl WorkflowApplication {
    pub(in crate::context) async fn workflow_runs(
        &self,
        input: ListWorkflowRunsInput,
    ) -> Result<Vec<WorkflowRunSummary>, AppError> {
        a4n_db::workflow::list_workflow_runs(&self.database, input)
            .await
            .map_err(workflow_read_app_error)
    }

    pub(in crate::context) async fn workflow_run_summary(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowRunSummary, AppError> {
        a4n_db::workflow::workflow_run_summary_by_id(&self.database, workflow_run_id)
            .await
            .map_err(workflow_read_app_error)
    }

    pub(in crate::context) async fn workflow_runs_by_project_kind(
        &self,
        project_id: i64,
        workflow_kind: &str,
    ) -> Result<Vec<WorkflowRunSummary>, AppError> {
        a4n_db::workflow::workflow_runs_by_project_kind(&self.database, project_id, workflow_kind)
            .await
            .map_err(workflow_read_app_error)
    }

    pub(in crate::context) async fn workflow_run_status_record_by_id(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowRunStatusRecord, AppError> {
        a4n_db::workflow::workflow_run_status_record_by_id(&self.database, workflow_run_id)
            .await
            .map_err(workflow_read_app_error)
    }

    pub(in crate::context) async fn workflow_run_statuses_by_ids(
        &self,
        workflow_run_ids: &[i64],
    ) -> Result<Vec<(i64, String)>, AppError> {
        a4n_db::workflow::workflow_run_statuses_by_ids(&self.database, workflow_run_ids)
            .await
            .map_err(workflow_read_app_error)
    }

    pub(in crate::context) async fn latest_workflow_run_by_business_key(
        &self,
        business_key: &str,
    ) -> Result<Option<WorkflowRunSummary>, AppError> {
        a4n_db::workflow::latest_workflow_run_by_business_key(&self.database, business_key)
            .await
            .map_err(workflow_read_app_error)
    }

    pub(in crate::context) async fn latest_workflow_run_status_record_by_business_key(
        &self,
        business_key: &str,
    ) -> Result<Option<WorkflowRunStatusRecord>, AppError> {
        a4n_db::workflow::latest_workflow_run_status_record_by_business_key(
            &self.database,
            business_key,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    pub(in crate::context) async fn latest_workflow_run_recovery_records_by_business_keys(
        &self,
        business_keys: &[String],
    ) -> Result<Vec<WorkflowRunRecoveryRecord>, AppError> {
        a4n_db::workflow::latest_workflow_run_recovery_records_by_business_keys(
            &self.database,
            business_keys,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    pub(in crate::context) async fn workflow_checkpoint_summaries(
        &self,
        workflow_run_id: i64,
    ) -> Result<Vec<a4n_db::workflow::WorkflowCheckpointSummary>, AppError> {
        a4n_db::workflow::workflow_checkpoint_summaries(&self.database, workflow_run_id)
            .await
            .map_err(workflow_read_app_error)
    }

    pub(in crate::context) async fn workflow_checkpoint_summaries_for_runs(
        &self,
        workflow_run_ids: &[i64],
    ) -> Result<Vec<a4n_db::workflow::WorkflowCheckpointSummary>, AppError> {
        a4n_db::workflow::workflow_checkpoint_summaries_for_runs(&self.database, workflow_run_ids)
            .await
            .map_err(workflow_read_app_error)
    }

    pub(in crate::context) async fn workflow_checkpoint_statuses_for_runs(
        &self,
        workflow_run_ids: &[i64],
    ) -> Result<Vec<(i64, String)>, AppError> {
        a4n_db::workflow::workflow_checkpoint_statuses_for_runs(&self.database, workflow_run_ids)
            .await
            .map_err(workflow_read_app_error)
    }

    pub(in crate::context) async fn workflow_checkpoint_step_statuses(
        &self,
        workflow_run_id: i64,
    ) -> Result<Vec<(String, String)>, AppError> {
        a4n_db::workflow::workflow_checkpoint_step_statuses(&self.database, workflow_run_id)
            .await
            .map_err(workflow_read_app_error)
    }

    pub(in crate::context) async fn completed_workflow_checkpoint_by_fingerprint(
        &self,
        workflow_run_id: i64,
        step_key: &str,
        input_fingerprint_sha256: &str,
    ) -> Result<Option<a4n_db::workflow::WorkflowCheckpointSummary>, AppError> {
        a4n_db::workflow::completed_workflow_checkpoint_by_fingerprint(
            &self.database,
            workflow_run_id,
            step_key,
            input_fingerprint_sha256,
        )
        .await
        .map_err(workflow_read_app_error)
        .map(|checkpoint| checkpoint.map(a4n_db::workflow::workflow_checkpoint_summary_from_model))
    }

    /// 加载已完成的 LLM call checkpoint（含 fingerprint 计算与合同校验入口）。
    pub(in crate::context) async fn load_completed_llm_call_checkpoint(
        &self,
        workflow_run_id: i64,
        call_key: &str,
        input_fingerprint_json: &serde_json::Value,
    ) -> Result<Option<CompletedLlmCallCheckpoint>, AppError> {
        a4n_db::workflow::load_completed_llm_call_checkpoint(
            &self.database,
            workflow_run_id,
            call_key,
            input_fingerprint_json,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    pub(in crate::context) async fn validate_llm_call_output<T>(
        &self,
        workflow_run_id: i64,
        call_key: &str,
        input_fingerprint_json: &serde_json::Value,
        output: Result<T, AppError>,
    ) -> Result<T, AppError> {
        let error = match output {
            Ok(output) => return Ok(output),
            Err(error) => error,
        };
        if let Err(rejection_error) = a4n_db::workflow::reject_completed_llm_call_checkpoint(
            &self.database,
            workflow_run_id,
            call_key,
            input_fingerprint_json,
            &error.to_string(),
        )
        .await
        {
            return Err(AppError::InvalidState {
                current: format!("{error}；撤销无效调用断点失败：{rejection_error}"),
                operation: format!("校验 LLM 调用 {call_key}"),
            });
        }
        Err(error)
    }

    pub(in crate::context) async fn cancel_workflow_run_persistence(
        &self,
        workflow_run_id: i64,
        reason: &str,
    ) -> Result<CancelWorkflowRunPersistenceResult, AppError> {
        a4n_db::workflow::cancel_workflow_run_persistence(&self.database, workflow_run_id, reason)
            .await
            .map_err(workflow_write_app_error)
    }

    pub(in crate::context) async fn upsert_workflow_run(
        &self,
        input: UpsertWorkflowRunInput,
    ) -> Result<WorkflowRunSummary, AppError> {
        a4n_db::workflow::upsert_workflow_run(&self.database, input)
            .await
            .map_err(workflow_write_app_error)
    }

    pub(in crate::context) async fn resume_or_upsert_workflow_run(
        &self,
        input: UpsertWorkflowRunInput,
    ) -> Result<WorkflowRunSummary, AppError> {
        a4n_db::workflow::resume_or_upsert_workflow_run(&self.database, input)
            .await
            .map_err(workflow_write_app_error)
    }

    pub(in crate::context) async fn claim_exclusive_workflow_run(
        &self,
        input: ClaimExclusiveWorkflowRunInput,
    ) -> Result<WorkflowRunSummary, AppError> {
        a4n_db::workflow::claim_exclusive_workflow_run(&self.database, input)
            .await
            .map_err(workflow_write_app_error)
    }

    pub(in crate::context) async fn mark_workflow_status(
        &self,
        workflow_run_id: i64,
        status: &str,
        can_resume: bool,
        error_message: Option<&str>,
    ) -> Result<WorkflowRunSummary, AppError> {
        a4n_db::workflow::mark_workflow_status(
            &self.database,
            workflow_run_id,
            status,
            can_resume,
            error_message,
        )
        .await
        .map_err(workflow_write_app_error)
    }

    pub(in crate::context) async fn resume_workflow_run(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowRunSummary, AppError> {
        a4n_db::workflow::resume_workflow_run(&self.database, workflow_run_id)
            .await
            .map_err(workflow_write_app_error)
    }

    pub(in crate::context) async fn resume_workflow_run_with_active_task(
        &self,
        workflow_run_id: i64,
        active_task_id: &str,
    ) -> Result<WorkflowRunSummary, AppError> {
        a4n_db::workflow::resume_workflow_run_with_active_task(
            &self.database,
            workflow_run_id,
            active_task_id,
        )
        .await
        .map_err(workflow_write_app_error)
    }

    pub(in crate::context) async fn mark_workflow_current_step(
        &self,
        workflow_run_id: i64,
        current_step_key: &str,
    ) -> Result<WorkflowRunSummary, AppError> {
        a4n_db::workflow::mark_workflow_current_step(
            &self.database,
            workflow_run_id,
            current_step_key,
        )
        .await
        .map_err(workflow_write_app_error)
    }

    pub(in crate::context) async fn upsert_workflow_checkpoint(
        &self,
        input: UpsertWorkflowCheckpointInput,
    ) -> Result<workflow_checkpoint::Model, AppError> {
        a4n_db::workflow::upsert_workflow_checkpoint(&self.database, input)
            .await
            .map_err(workflow_write_app_error)
    }

    pub(in crate::context) async fn record_workflow_progress_event(
        &self,
        input: WorkflowProgressEventInput<'_>,
    ) -> Result<workflow_progress_event::Model, AppError> {
        a4n_db::workflow::record_workflow_progress_event(&self.database, input)
            .await
            .map_err(workflow_write_app_error)
    }

    pub(in crate::context) async fn persist_workflow_progress_snapshot(
        &self,
        workflow_run_id: i64,
        snapshot: serde_json::Value,
    ) -> Result<WorkflowRunSummary, AppError> {
        a4n_db::workflow::persist_workflow_progress_snapshot(
            &self.database,
            workflow_run_id,
            snapshot,
        )
        .await
        .map_err(workflow_write_app_error)
    }

    /// 保存已完成的 LLM call checkpoint。
    pub(in crate::context) async fn save_completed_llm_call_checkpoint(
        &self,
        input: CompletedLlmCallCheckpointInput,
    ) -> Result<(), AppError> {
        a4n_db::workflow::save_completed_llm_call_checkpoint(&self.database, input)
            .await
            .map_err(workflow_write_app_error)
    }

    pub(in crate::context) async fn delete_workflow_checkpoints_by_keys(
        &self,
        workflow_run_id: i64,
        step_keys: &[&str],
    ) -> Result<u64, AppError> {
        a4n_db::workflow::delete_workflow_checkpoints_by_keys(
            &self.database,
            workflow_run_id,
            step_keys,
        )
        .await
        .map_err(workflow_write_app_error)
    }
}
