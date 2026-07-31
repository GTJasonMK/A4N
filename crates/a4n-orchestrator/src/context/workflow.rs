//! Workflow 领域私有 owner。
//!
//! 持有 `DbHandle`，封装 workflow run / checkpoint / progress / task 历史的读写。
//! 编排侧只通过 `OrchestratorContext` 行为方法或 runtime 窄 facade 访问。

use a4n_core::error::AppError;
use a4n_core::models::generation::{workflow_checkpoint, workflow_progress_event, workflow_run};
use a4n_db::DbHandle;

use super::OrchestratorContext;
use crate::workflow::{
    CancelWorkflowRunPersistenceResult, ClaimExclusiveWorkflowRunInput,
    CompletedLlmCallCheckpointInput, GetWorkflowRunInput, ListWorkflowRunsInput, PersistedTaskInfo,
    TaskProgressEventSummary, UpsertWorkflowCheckpointInput, UpsertWorkflowRunInput,
    WorkflowProgressEventInput, WorkflowRunDetailSummary, WorkflowRunSummary,
    is_video_status_final_for_cancel_interrupt, workflow_read_app_error, workflow_write_app_error,
};

/// Workflow 领域 application owner。
#[derive(Clone)]
pub(super) struct WorkflowApplication {
    database: DbHandle,
}

impl WorkflowApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn workflow_runs(
        &self,
        input: ListWorkflowRunsInput,
    ) -> Result<Vec<WorkflowRunSummary>, AppError> {
        a4n_db::workflow::list_workflow_runs(self.database.connection(), input)
            .await
            .map_err(workflow_read_app_error)
    }

    async fn workflow_run_summary(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowRunSummary, AppError> {
        a4n_db::workflow::workflow_run_summary_by_id(self.database.connection(), workflow_run_id)
            .await
            .map_err(workflow_read_app_error)
    }

    async fn workflow_run_detail(
        &self,
        input: GetWorkflowRunInput,
    ) -> Result<WorkflowRunDetailSummary, AppError> {
        a4n_db::workflow::workflow_run_detail(self.database.connection(), input)
            .await
            .map_err(workflow_read_app_error)
    }

    async fn workflow_progress_events(
        &self,
        workflow_run_id: i64,
        limit: usize,
    ) -> Result<Vec<TaskProgressEventSummary>, AppError> {
        a4n_db::workflow::workflow_progress_events(
            self.database.connection(),
            workflow_run_id,
            limit,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    async fn persisted_task_history(
        &self,
        project_id: Option<i64>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<PersistedTaskInfo>, AppError> {
        a4n_db::workflow::persisted_task_history(
            self.database.connection(),
            project_id,
            offset,
            limit,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    async fn persisted_task_by_id(
        &self,
        task_id: &str,
    ) -> Result<Option<PersistedTaskInfo>, AppError> {
        a4n_db::workflow::persisted_task_by_id(self.database.connection(), task_id)
            .await
            .map_err(workflow_read_app_error)
    }

    async fn workflow_run_model_by_active_task_id(
        &self,
        task_id: &str,
    ) -> Result<Option<workflow_run::Model>, AppError> {
        a4n_db::workflow::workflow_run_model_by_active_task_id(self.database.connection(), task_id)
            .await
            .map_err(workflow_read_app_error)
    }

    async fn workflow_run_models(
        &self,
        input: ListWorkflowRunsInput,
    ) -> Result<Vec<workflow_run::Model>, AppError> {
        a4n_db::workflow::list_workflow_run_models(self.database.connection(), input)
            .await
            .map_err(workflow_read_app_error)
    }

    async fn workflow_runs_by_project_kind(
        &self,
        project_id: i64,
        workflow_kind: &str,
    ) -> Result<Vec<workflow_run::Model>, AppError> {
        a4n_db::workflow::workflow_runs_by_project_kind(
            self.database.connection(),
            project_id,
            workflow_kind,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    async fn workflow_run_model_by_id(
        &self,
        workflow_run_id: i64,
    ) -> Result<workflow_run::Model, AppError> {
        a4n_db::workflow::workflow_run_model_by_id(self.database.connection(), workflow_run_id)
            .await
            .map_err(workflow_read_app_error)
    }

    async fn workflow_run_model_optional_by_id(
        &self,
        workflow_run_id: i64,
    ) -> Result<Option<workflow_run::Model>, AppError> {
        a4n_db::workflow::workflow_run_model_optional_by_id(
            self.database.connection(),
            workflow_run_id,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    async fn latest_workflow_run_by_business_key(
        &self,
        business_key: &str,
    ) -> Result<Option<workflow_run::Model>, AppError> {
        a4n_db::workflow::latest_workflow_run_by_business_key(
            self.database.connection(),
            business_key,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    async fn workflow_checkpoint_models(
        &self,
        workflow_run_id: i64,
    ) -> Result<Vec<workflow_checkpoint::Model>, AppError> {
        a4n_db::workflow::workflow_checkpoint_models(self.database.connection(), workflow_run_id)
            .await
            .map_err(workflow_read_app_error)
    }

    async fn workflow_checkpoint_model_by_step(
        &self,
        workflow_run_id: i64,
        step_key: &str,
    ) -> Result<Option<workflow_checkpoint::Model>, AppError> {
        a4n_db::workflow::workflow_checkpoint_model_by_step(
            self.database.connection(),
            workflow_run_id,
            step_key,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    async fn completed_workflow_checkpoint_by_fingerprint(
        &self,
        workflow_run_id: i64,
        step_key: &str,
        input_fingerprint_sha256: &str,
    ) -> Result<Option<workflow_checkpoint::Model>, AppError> {
        a4n_db::workflow::completed_workflow_checkpoint_by_fingerprint(
            self.database.connection(),
            workflow_run_id,
            step_key,
            input_fingerprint_sha256,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    /// 加载已完成的 LLM call checkpoint（含 fingerprint 计算与合同校验入口）。
    async fn load_completed_llm_call_checkpoint(
        &self,
        workflow_run_id: i64,
        call_key: &str,
        input_fingerprint_json: &serde_json::Value,
    ) -> Result<Option<workflow_checkpoint::Model>, AppError> {
        a4n_db::workflow::load_completed_llm_call_checkpoint(
            self.database.connection(),
            workflow_run_id,
            call_key,
            input_fingerprint_json,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    async fn cancel_workflow_run_persistence(
        &self,
        workflow_run_id: i64,
        reason: &str,
    ) -> Result<CancelWorkflowRunPersistenceResult, AppError> {
        a4n_db::workflow::cancel_workflow_run_persistence(
            self.database.connection(),
            workflow_run_id,
            reason,
        )
        .await
        .map_err(workflow_write_app_error)
    }

    async fn upsert_workflow_run(
        &self,
        input: UpsertWorkflowRunInput,
    ) -> Result<workflow_run::Model, AppError> {
        a4n_db::workflow::upsert_workflow_run(self.database.connection(), input)
            .await
            .map_err(workflow_write_app_error)
    }

    async fn claim_exclusive_workflow_run(
        &self,
        input: ClaimExclusiveWorkflowRunInput,
    ) -> Result<workflow_run::Model, AppError> {
        a4n_db::workflow::claim_exclusive_workflow_run(self.database.connection(), input)
            .await
            .map_err(workflow_write_app_error)
    }

    async fn mark_workflow_status(
        &self,
        workflow_run_id: i64,
        status: &str,
        can_resume: bool,
        error_message: Option<&str>,
    ) -> Result<workflow_run::Model, AppError> {
        a4n_db::workflow::mark_workflow_status(
            self.database.connection(),
            workflow_run_id,
            status,
            can_resume,
            error_message,
        )
        .await
        .map_err(workflow_write_app_error)
    }

    async fn resume_workflow_run(
        &self,
        workflow_run_id: i64,
    ) -> Result<workflow_run::Model, AppError> {
        a4n_db::workflow::resume_workflow_run(self.database.connection(), workflow_run_id)
            .await
            .map_err(workflow_write_app_error)
    }

    async fn mark_workflow_current_step(
        &self,
        workflow_run_id: i64,
        current_step_key: &str,
    ) -> Result<workflow_run::Model, AppError> {
        a4n_db::workflow::mark_workflow_current_step(
            self.database.connection(),
            workflow_run_id,
            current_step_key,
        )
        .await
        .map_err(workflow_write_app_error)
    }

    async fn upsert_workflow_checkpoint(
        &self,
        input: UpsertWorkflowCheckpointInput,
    ) -> Result<workflow_checkpoint::Model, AppError> {
        a4n_db::workflow::upsert_workflow_checkpoint(self.database.connection(), input)
            .await
            .map_err(workflow_write_app_error)
    }

    async fn update_workflow_checkpoint_artifacts(
        &self,
        workflow_run_id: i64,
        step_key: &str,
        artifacts_json: serde_json::Value,
    ) -> Result<workflow_checkpoint::Model, AppError> {
        a4n_db::workflow::update_workflow_checkpoint_artifacts(
            self.database.connection(),
            workflow_run_id,
            step_key,
            artifacts_json,
        )
        .await
        .map_err(workflow_write_app_error)
    }

    async fn record_workflow_progress_event(
        &self,
        input: WorkflowProgressEventInput<'_>,
    ) -> Result<workflow_progress_event::Model, AppError> {
        a4n_db::workflow::record_workflow_progress_event(self.database.connection(), input)
            .await
            .map_err(workflow_write_app_error)
    }

    /// 保存已完成的 LLM call checkpoint。
    async fn save_completed_llm_call_checkpoint(
        &self,
        input: CompletedLlmCallCheckpointInput,
    ) -> Result<(), AppError> {
        a4n_db::workflow::save_completed_llm_call_checkpoint(self.database.connection(), input)
            .await
            .map_err(workflow_write_app_error)
    }

    async fn delete_workflow_checkpoints_by_keys(
        &self,
        workflow_run_id: i64,
        step_keys: &[&str],
    ) -> Result<u64, AppError> {
        a4n_db::workflow::delete_workflow_checkpoints_by_keys(
            self.database.connection(),
            workflow_run_id,
            step_keys,
        )
        .await
        .map_err(workflow_write_app_error)
    }
}

impl OrchestratorContext {
    pub async fn workflow_runs(
        &self,
        input: ListWorkflowRunsInput,
    ) -> Result<Vec<WorkflowRunSummary>, AppError> {
        self.workflow.workflow_runs(input).await
    }

    pub async fn workflow_run_summary(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowRunSummary, AppError> {
        self.workflow.workflow_run_summary(workflow_run_id).await
    }

    pub async fn workflow_run_detail(
        &self,
        input: GetWorkflowRunInput,
    ) -> Result<WorkflowRunDetailSummary, AppError> {
        self.workflow.workflow_run_detail(input).await
    }

    pub async fn workflow_progress_events(
        &self,
        workflow_run_id: i64,
        limit: usize,
    ) -> Result<Vec<TaskProgressEventSummary>, AppError> {
        self.workflow
            .workflow_progress_events(workflow_run_id, limit)
            .await
    }

    pub async fn persisted_task_history(
        &self,
        project_id: Option<i64>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<PersistedTaskInfo>, AppError> {
        self.workflow
            .persisted_task_history(project_id, offset, limit)
            .await
    }

    pub async fn persisted_task_by_id(
        &self,
        task_id: &str,
    ) -> Result<Option<PersistedTaskInfo>, AppError> {
        self.workflow.persisted_task_by_id(task_id).await
    }

    pub async fn workflow_run_model_by_active_task_id(
        &self,
        task_id: &str,
    ) -> Result<Option<workflow_run::Model>, AppError> {
        self.workflow
            .workflow_run_model_by_active_task_id(task_id)
            .await
    }

    pub async fn workflow_run_models(
        &self,
        input: ListWorkflowRunsInput,
    ) -> Result<Vec<workflow_run::Model>, AppError> {
        self.workflow.workflow_run_models(input).await
    }

    pub async fn workflow_runs_by_project_kind(
        &self,
        project_id: i64,
        workflow_kind: &str,
    ) -> Result<Vec<workflow_run::Model>, AppError> {
        self.workflow
            .workflow_runs_by_project_kind(project_id, workflow_kind)
            .await
    }

    pub async fn workflow_run_model_by_id(
        &self,
        workflow_run_id: i64,
    ) -> Result<workflow_run::Model, AppError> {
        self.workflow
            .workflow_run_model_by_id(workflow_run_id)
            .await
    }

    pub async fn workflow_run_model_optional_by_id(
        &self,
        workflow_run_id: i64,
    ) -> Result<Option<workflow_run::Model>, AppError> {
        self.workflow
            .workflow_run_model_optional_by_id(workflow_run_id)
            .await
    }

    pub async fn latest_workflow_run_by_business_key(
        &self,
        business_key: &str,
    ) -> Result<Option<workflow_run::Model>, AppError> {
        self.workflow
            .latest_workflow_run_by_business_key(business_key)
            .await
    }

    pub async fn workflow_checkpoint_models(
        &self,
        workflow_run_id: i64,
    ) -> Result<Vec<workflow_checkpoint::Model>, AppError> {
        self.workflow
            .workflow_checkpoint_models(workflow_run_id)
            .await
    }

    pub async fn workflow_checkpoint_model_by_step(
        &self,
        workflow_run_id: i64,
        step_key: &str,
    ) -> Result<Option<workflow_checkpoint::Model>, AppError> {
        self.workflow
            .workflow_checkpoint_model_by_step(workflow_run_id, step_key)
            .await
    }

    pub async fn completed_workflow_checkpoint_by_fingerprint(
        &self,
        workflow_run_id: i64,
        step_key: &str,
        input_fingerprint_sha256: &str,
    ) -> Result<Option<workflow_checkpoint::Model>, AppError> {
        self.workflow
            .completed_workflow_checkpoint_by_fingerprint(
                workflow_run_id,
                step_key,
                input_fingerprint_sha256,
            )
            .await
    }

    /// 加载已完成的 LLM call checkpoint（含 fingerprint 计算与合同校验入口）。
    pub async fn load_completed_llm_call_checkpoint(
        &self,
        workflow_run_id: i64,
        call_key: &str,
        input_fingerprint_json: &serde_json::Value,
    ) -> Result<Option<workflow_checkpoint::Model>, AppError> {
        self.workflow
            .load_completed_llm_call_checkpoint(workflow_run_id, call_key, input_fingerprint_json)
            .await
    }

    pub async fn cancel_workflow_run_and_interrupt_videos(
        &self,
        workflow_run_id: i64,
        reason: &str,
    ) -> Result<CancelWorkflowRunPersistenceResult, AppError> {
        let cancellation = self
            .workflow
            .cancel_workflow_run_persistence(workflow_run_id, reason)
            .await?;
        if !cancellation.already_terminal {
            if let Some(task_id) = cancellation.active_task_id.as_deref() {
                self.cancel_task_handle(task_id);
                if let Err(error) = self.queue_cancel(task_id).await {
                    tracing::warn!(
                        workflow_run_id,
                        task_id,
                        error = %error,
                        "workflow 已持久化取消，但活动队列任务未能同步取消"
                    );
                }
            }
            self.interrupt_generated_videos_for_cancelled_workflow(
                &cancellation.interrupted_generated_video_ids,
                reason,
            )
            .await?;
        }
        Ok(cancellation)
    }

    async fn interrupt_generated_videos_for_cancelled_workflow(
        &self,
        video_ids: &[i64],
        reason: &str,
    ) -> Result<(), AppError> {
        for video_id in video_ids {
            let video = match self.video.generated_video_by_id(*video_id).await {
                Ok(Some(video)) => video,
                Ok(None) => {
                    tracing::warn!(video_id, "取消工作流时读取关联视频失败: 视频不存在");
                    continue;
                }
                Err(err) => {
                    tracing::warn!(video_id, error = %err, "取消工作流时读取关联视频失败");
                    continue;
                }
            };
            if is_video_status_final_for_cancel_interrupt(&video.status) {
                continue;
            }
            if crate::video::normalized_provider_job_id(video.provider_job_id.as_deref()).is_some()
            {
                self.video
                    .mark_generated_video_error_status_and_prompt_record(
                        video.id,
                        crate::video::prompt_id_from_generated_video(&video),
                        "interrupted",
                        reason,
                    )
                    .await?;
            } else {
                self.mark_generated_video_cancelled_validated(
                    video.id,
                    crate::video::prompt_id_from_generated_video(&video),
                    reason,
                )
                .await?;
            }
        }
        Ok(())
    }

    pub async fn upsert_workflow_run(
        &self,
        input: UpsertWorkflowRunInput,
    ) -> Result<workflow_run::Model, AppError> {
        self.workflow.upsert_workflow_run(input).await
    }

    pub async fn update_workflow_checkpoint_artifacts(
        &self,
        workflow_run_id: i64,
        step_key: &str,
        artifacts_json: serde_json::Value,
    ) -> Result<workflow_checkpoint::Model, AppError> {
        self.workflow
            .update_workflow_checkpoint_artifacts(workflow_run_id, step_key, artifacts_json)
            .await
    }

    pub async fn claim_exclusive_workflow_run(
        &self,
        input: ClaimExclusiveWorkflowRunInput,
    ) -> Result<workflow_run::Model, AppError> {
        self.workflow.claim_exclusive_workflow_run(input).await
    }

    pub async fn mark_workflow_status(
        &self,
        workflow_run_id: i64,
        status: &str,
        can_resume: bool,
        error_message: Option<&str>,
    ) -> Result<workflow_run::Model, AppError> {
        self.workflow
            .mark_workflow_status(workflow_run_id, status, can_resume, error_message)
            .await
    }

    /// 开始工作流：校验 business_key/kind/id 与 RUNNING 后 upsert。
    /// Host 仍可构造指纹与业务字段，但状态推进 owner 在 orch。
    pub async fn begin_workflow_run_validated(
        &self,
        input: UpsertWorkflowRunInput,
    ) -> Result<workflow_run::Model, AppError> {
        crate::workflow::validate_begin_workflow_run_input(&input)
            .map_err(AppError::InvalidArgument)?;
        self.upsert_workflow_run(input).await
    }

    /// 按成败将 workflow 标为 completed/failed。
    pub async fn finalize_workflow_run_validated(
        &self,
        workflow_run_id: i64,
        success: bool,
        error_message: Option<&str>,
    ) -> Result<workflow_run::Model, AppError> {
        crate::workflow::require_positive_workflow_run_id(workflow_run_id)
            .map_err(AppError::InvalidArgument)?;
        let (status, can_resume) =
            crate::workflow::workflow_run_finalize_status_from_success(success);
        let message = if success {
            None
        } else {
            error_message
                .map(str::trim)
                .filter(|msg| !msg.is_empty())
                .or(Some("工作流失败"))
        };
        self.mark_workflow_status(workflow_run_id, status, can_resume, message)
            .await
    }

    pub(crate) async fn finalize_workflow_result<T>(
        &self,
        workflow_run_id: i64,
        result: Result<T, AppError>,
        completed_context: Option<&str>,
    ) -> Result<T, AppError> {
        let outcome = match &result {
            Ok(_) => crate::workflow::WorkflowRunFinalizeOutcome::Completed,
            Err(AppError::Cancelled) => crate::workflow::WorkflowRunFinalizeOutcome::Cancelled,
            Err(_) => crate::workflow::WorkflowRunFinalizeOutcome::Failed,
        };
        let error_message = result.as_ref().err().map(ToString::to_string);
        let finalized = self
            .finalize_workflow_run_outcome_validated(
                workflow_run_id,
                outcome,
                error_message.as_deref(),
            )
            .await;
        if matches!(&finalized, Err(AppError::Cancelled)) {
            return Err(AppError::Cancelled);
        }
        match (result, finalized) {
            (Ok(value), Ok(_)) => Ok(value),
            (Err(error), Ok(_)) => Err(error),
            (Ok(_), Err(status_error)) => match completed_context {
                Some(context) => Err(AppError::Internal(anyhow::anyhow!(
                    "{context}，但工作流状态写入失败: {status_error}"
                ))),
                None => Err(status_error),
            },
            (Err(primary), Err(status_error)) => Err(AppError::Internal(anyhow::anyhow!(
                "{primary}；工作流失败状态写入失败: {status_error}"
            ))),
        }
    }

    pub(crate) async fn finalize_workflow_outcome_result<T>(
        &self,
        workflow_run_id: i64,
        result: Result<T, AppError>,
        failure_context: &str,
    ) -> Result<T, AppError> {
        let outcome = match &result {
            Ok(_) => crate::workflow::WorkflowRunFinalizeOutcome::Completed,
            Err(AppError::Cancelled) => crate::workflow::WorkflowRunFinalizeOutcome::Cancelled,
            Err(_) => crate::workflow::WorkflowRunFinalizeOutcome::Failed,
        };
        let error_message = result.as_ref().err().map(ToString::to_string);
        let finalized = self
            .finalize_workflow_run_outcome_validated(
                workflow_run_id,
                outcome,
                error_message.as_deref(),
            )
            .await;
        if matches!(&finalized, Err(AppError::Cancelled)) {
            return Err(AppError::Cancelled);
        }
        match (result, finalized) {
            (Ok(value), Ok(_)) => Ok(value),
            (Ok(_), Err(status_error)) => Err(status_error),
            (Err(error), Ok(_)) => Err(error),
            (Err(error), Err(status_error)) => Err(AppError::InvalidState {
                current: error.to_string(),
                operation: format!(
                    "{failure_context}失败，且 workflow 终态写入失败: {status_error}"
                ),
            }),
        }
    }

    /// 按终态枚举（含 cancelled）推进 workflow。
    pub async fn finalize_workflow_run_outcome_validated(
        &self,
        workflow_run_id: i64,
        outcome: crate::workflow::WorkflowRunFinalizeOutcome,
        error_message: Option<&str>,
    ) -> Result<workflow_run::Model, AppError> {
        crate::workflow::require_positive_workflow_run_id(workflow_run_id)
            .map_err(AppError::InvalidArgument)?;
        let (status, can_resume) = crate::workflow::workflow_run_finalize_status(outcome);
        let message = match outcome {
            crate::workflow::WorkflowRunFinalizeOutcome::Completed => None,
            crate::workflow::WorkflowRunFinalizeOutcome::Failed
            | crate::workflow::WorkflowRunFinalizeOutcome::Cancelled => error_message
                .map(str::trim)
                .filter(|msg| !msg.is_empty())
                .or(Some("工作流未完成")),
        };
        self.mark_workflow_status(workflow_run_id, status, can_resume, message)
            .await
    }

    /// 将 workflow 标为 completed（不可 resume）。
    pub async fn complete_workflow_run_validated(
        &self,
        workflow_run_id: i64,
    ) -> Result<workflow_run::Model, AppError> {
        self.finalize_workflow_run_outcome_validated(
            workflow_run_id,
            crate::workflow::WorkflowRunFinalizeOutcome::Completed,
            None,
        )
        .await
    }

    /// 将 workflow 标为 failed（可 resume）。
    pub async fn fail_workflow_run_validated(
        &self,
        workflow_run_id: i64,
        error_message: Option<&str>,
    ) -> Result<workflow_run::Model, AppError> {
        self.finalize_workflow_run_outcome_validated(
            workflow_run_id,
            crate::workflow::WorkflowRunFinalizeOutcome::Failed,
            error_message,
        )
        .await
    }

    /// 将 workflow 标为 cancelled（可 resume）。
    pub async fn cancel_workflow_run_status_validated(
        &self,
        workflow_run_id: i64,
        error_message: Option<&str>,
    ) -> Result<workflow_run::Model, AppError> {
        self.finalize_workflow_run_outcome_validated(
            workflow_run_id,
            crate::workflow::WorkflowRunFinalizeOutcome::Cancelled,
            error_message,
        )
        .await
    }

    /// 将 workflow 重新标为 running（如 resume 续跑）。
    pub async fn mark_workflow_running_validated(
        &self,
        workflow_run_id: i64,
    ) -> Result<workflow_run::Model, AppError> {
        crate::workflow::require_positive_workflow_run_id(workflow_run_id)
            .map_err(AppError::InvalidArgument)?;
        self.workflow.resume_workflow_run(workflow_run_id).await
    }

    pub async fn mark_workflow_current_step(
        &self,
        workflow_run_id: i64,
        current_step_key: &str,
    ) -> Result<workflow_run::Model, AppError> {
        self.workflow
            .mark_workflow_current_step(workflow_run_id, current_step_key)
            .await
    }

    pub async fn upsert_workflow_checkpoint(
        &self,
        input: UpsertWorkflowCheckpointInput,
    ) -> Result<workflow_checkpoint::Model, AppError> {
        self.workflow.upsert_workflow_checkpoint(input).await
    }

    pub async fn record_workflow_progress_event(
        &self,
        input: WorkflowProgressEventInput<'_>,
    ) -> Result<workflow_progress_event::Model, AppError> {
        self.workflow.record_workflow_progress_event(input).await
    }

    /// 保存已完成的 LLM call checkpoint。
    pub async fn save_completed_llm_call_checkpoint(
        &self,
        input: CompletedLlmCallCheckpointInput,
    ) -> Result<(), AppError> {
        self.workflow
            .save_completed_llm_call_checkpoint(input)
            .await
    }

    pub async fn delete_workflow_checkpoints_by_keys(
        &self,
        workflow_run_id: i64,
        step_keys: &[&str],
    ) -> Result<u64, AppError> {
        self.workflow
            .delete_workflow_checkpoints_by_keys(workflow_run_id, step_keys)
            .await
    }
}
