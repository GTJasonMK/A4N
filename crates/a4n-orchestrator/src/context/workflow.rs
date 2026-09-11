//! Workflow 领域私有 owner。
//!
//! 持有 `DbHandle`，封装 workflow run / checkpoint / progress / task 历史的读写。
//! 编排侧只通过 `OrchestratorContext` 行为方法或 runtime 窄 facade 访问。

use std::collections::HashSet;

use a4n_api_contract::workflow::{
    ActiveTaskLookupRecord, TaskLookupRecord, WorkflowCancellationPort, WorkflowResumeRecord,
    WorkflowTaskAccessRecord, WorkflowTaskLookupApplicationPort,
};
use a4n_db::DbHandle;
use a4n_db_models::models::generation::{
    workflow_checkpoint, workflow_progress_event, workflow_run,
};
use a4n_error::AppError;

use super::OrchestratorContext;
use crate::workflow::{
    CancelWorkflowRunPersistenceResult, ClaimExclusiveWorkflowRunInput, CompletedLlmCallCheckpoint,
    CompletedLlmCallCheckpointInput, GetWorkflowRunInput, ListWorkflowRunsInput, PersistedTaskInfo,
    TaskProgressEventSummary, UpsertWorkflowCheckpointInput, UpsertWorkflowRunInput,
    WorkflowProgressEventInput, WorkflowRunAccessRecord, WorkflowRunDetailSummary,
    WorkflowRunRecoveryRecord, WorkflowRunStatusRecord, WorkflowRunSummary,
    is_video_status_final_for_cancel_interrupt, workflow_read_app_error,
    workflow_run_summary_to_api, workflow_write_app_error,
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

    async fn task_history(
        &self,
        project_id: Option<i64>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<a4n_api_contract::task::TaskInfoSummary>, AppError> {
        Ok(self
            .persisted_task_history(project_id, offset, limit)
            .await?
            .into_iter()
            .map(|item| task_info_summary(&item.task))
            .collect())
    }

    async fn persisted_task_by_id(
        &self,
        task_id: &str,
    ) -> Result<Option<PersistedTaskInfo>, AppError> {
        a4n_db::workflow::persisted_task_by_id(self.database.connection(), task_id)
            .await
            .map_err(workflow_read_app_error)
    }

    async fn active_tasks(&self) -> Result<Vec<ActiveTaskLookupRecord>, AppError> {
        self.persisted_task_history(None, 0, 100)
            .await?
            .into_iter()
            .map(active_task_lookup_record)
            .collect()
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

    async fn workflow_run_resume_record(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowResumeRecord, AppError> {
        let run = self.workflow_run_model_by_id(workflow_run_id).await?;
        Ok(WorkflowResumeRecord {
            id: run.id,
            workflow_kind: run.workflow_kind,
            project_id: run.project_id,
            root_project_id: run.root_project_id,
            chapter_id: run.chapter_id,
            scope_table: run.scope_table,
            scope_id: run.scope_id,
            input_fingerprint_json: run.input_fingerprint_json,
            status: run.status,
            can_resume: run.can_resume,
            resume_block_reason: run.resume_block_reason,
        })
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

    async fn workflow_run_status_record_by_id(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowRunStatusRecord, AppError> {
        a4n_db::workflow::workflow_run_status_record_by_id(
            self.database.connection(),
            workflow_run_id,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    async fn workflow_run_access_record_by_active_task_id(
        &self,
        task_id: &str,
    ) -> Result<Option<WorkflowRunAccessRecord>, AppError> {
        a4n_db::workflow::workflow_run_access_record_by_active_task_id(
            self.database.connection(),
            task_id,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    async fn workflow_run_statuses_by_ids(
        &self,
        workflow_run_ids: &[i64],
    ) -> Result<Vec<(i64, String)>, AppError> {
        a4n_db::workflow::workflow_run_statuses_by_ids(self.database.connection(), workflow_run_ids)
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

    async fn latest_workflow_run_status_record_by_business_key(
        &self,
        business_key: &str,
    ) -> Result<Option<WorkflowRunStatusRecord>, AppError> {
        a4n_db::workflow::latest_workflow_run_status_record_by_business_key(
            self.database.connection(),
            business_key,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    async fn latest_workflow_run_recovery_records_by_business_keys(
        &self,
        business_keys: &[String],
    ) -> Result<Vec<WorkflowRunRecoveryRecord>, AppError> {
        a4n_db::workflow::latest_workflow_run_recovery_records_by_business_keys(
            self.database.connection(),
            business_keys,
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

    async fn workflow_checkpoint_models_for_runs(
        &self,
        workflow_run_ids: &[i64],
    ) -> Result<Vec<workflow_checkpoint::Model>, AppError> {
        a4n_db::workflow::workflow_checkpoint_models_for_runs(
            self.database.connection(),
            workflow_run_ids,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    async fn workflow_checkpoint_statuses_for_runs(
        &self,
        workflow_run_ids: &[i64],
    ) -> Result<Vec<(i64, String)>, AppError> {
        a4n_db::workflow::workflow_checkpoint_statuses_for_runs(
            self.database.connection(),
            workflow_run_ids,
        )
        .await
        .map_err(workflow_read_app_error)
    }

    async fn workflow_checkpoint_step_statuses(
        &self,
        workflow_run_id: i64,
    ) -> Result<Vec<(String, String)>, AppError> {
        a4n_db::workflow::workflow_checkpoint_step_statuses(
            self.database.connection(),
            workflow_run_id,
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
    ) -> Result<Option<CompletedLlmCallCheckpoint>, AppError> {
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

    async fn resume_or_upsert_workflow_run(
        &self,
        input: UpsertWorkflowRunInput,
    ) -> Result<workflow_run::Model, AppError> {
        a4n_db::workflow::resume_or_upsert_workflow_run(self.database.connection(), input)
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

    async fn resume_workflow_run_with_active_task(
        &self,
        workflow_run_id: i64,
        active_task_id: &str,
    ) -> Result<workflow_run::Model, AppError> {
        a4n_db::workflow::resume_workflow_run_with_active_task(
            self.database.connection(),
            workflow_run_id,
            active_task_id,
        )
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

    pub(crate) async fn workflow_checkpoint_statuses_for_runs(
        &self,
        workflow_run_ids: &[i64],
    ) -> Result<Vec<(i64, String)>, AppError> {
        self.workflow
            .workflow_checkpoint_statuses_for_runs(workflow_run_ids)
            .await
    }

    pub(crate) async fn workflow_checkpoint_step_statuses(
        &self,
        workflow_run_id: i64,
    ) -> Result<Vec<(String, String)>, AppError> {
        self.workflow
            .workflow_checkpoint_step_statuses(workflow_run_id)
            .await
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

    pub(crate) async fn workflow_run_status_record_by_id(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowRunStatusRecord, AppError> {
        self.workflow
            .workflow_run_status_record_by_id(workflow_run_id)
            .await
    }

    pub async fn workflow_run_access_record_by_active_task_id(
        &self,
        task_id: &str,
    ) -> Result<Option<WorkflowRunAccessRecord>, AppError> {
        self.workflow
            .workflow_run_access_record_by_active_task_id(task_id)
            .await
    }

    pub(crate) async fn workflow_run_statuses_by_ids(
        &self,
        workflow_run_ids: &[i64],
    ) -> Result<Vec<(i64, String)>, AppError> {
        self.workflow
            .workflow_run_statuses_by_ids(workflow_run_ids)
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

    pub(crate) async fn latest_workflow_run_status_record_by_business_key(
        &self,
        business_key: &str,
    ) -> Result<Option<WorkflowRunStatusRecord>, AppError> {
        self.workflow
            .latest_workflow_run_status_record_by_business_key(business_key)
            .await
    }

    pub(crate) async fn latest_workflow_run_recovery_records_by_business_keys(
        &self,
        business_keys: &[String],
    ) -> Result<Vec<WorkflowRunRecoveryRecord>, AppError> {
        self.workflow
            .latest_workflow_run_recovery_records_by_business_keys(business_keys)
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

    pub async fn workflow_checkpoint_models_for_runs(
        &self,
        workflow_run_ids: &[i64],
    ) -> Result<Vec<workflow_checkpoint::Model>, AppError> {
        self.workflow
            .workflow_checkpoint_models_for_runs(workflow_run_ids)
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
    ) -> Result<Option<CompletedLlmCallCheckpoint>, AppError> {
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
        if video_ids.is_empty() {
            return Ok(());
        }
        let videos = self.video.generated_videos_by_ids(video_ids).await?;
        let videos_by_id = videos
            .into_iter()
            .map(|video| (video.id, video))
            .collect::<std::collections::HashMap<_, _>>();
        let mut inputs = Vec::with_capacity(video_ids.len());
        for video_id in video_ids {
            let Some(video) = videos_by_id.get(video_id) else {
                tracing::warn!(video_id, "取消工作流时读取关联视频失败: 视频不存在");
                continue;
            };
            if is_video_status_final_for_cancel_interrupt(&video.status) {
                continue;
            }
            inputs.push(a4n_db::video::GeneratedVideoErrorStatusBatchInput {
                video_id: video.id,
                prompt_id: a4n_domain::video::generated_video_prompt_id(
                    video.metadata_json.as_ref(),
                ),
                status: if crate::video::normalized_provider_job_id(
                    video.provider_job_id.as_deref(),
                )
                .is_some()
                {
                    a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED.to_string()
                } else {
                    a4n_domain::video_workflow::VIDEO_STATUS_CANCELLED.to_string()
                },
            });
        }
        let missing = self
            .video
            .mark_generated_videos_error_status_and_prompt_records(&inputs, reason)
            .await?;
        for video_id in missing {
            tracing::warn!(video_id, "取消工作流时读取关联视频失败: 视频不存在");
        }
        let cancelled_video_ids = inputs
            .iter()
            .filter(|input| input.status == a4n_domain::video_workflow::VIDEO_STATUS_CANCELLED)
            .map(|input| input.video_id)
            .collect::<Vec<_>>();
        if cancelled_video_ids.is_empty() {
            return Ok(());
        }
        let cancelled_run_counts = self
            .generation
            .cancel_generation_runs_for_targets(
                "generated_video",
                &cancelled_video_ids,
                reason.to_string(),
            )
            .await?
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        for input in inputs
            .iter()
            .filter(|input| input.status == a4n_domain::video_workflow::VIDEO_STATUS_CANCELLED)
        {
            crate::video::ensure_provider_generation_run_updated(
                input.video_id,
                cancelled_run_counts
                    .get(&input.video_id)
                    .copied()
                    .unwrap_or_default(),
                "取消 provider 视频生成",
            )
            .map_err(AppError::InvalidArgument)?;
        }
        Ok(())
    }

    pub async fn upsert_workflow_run(
        &self,
        input: UpsertWorkflowRunInput,
    ) -> Result<workflow_run::Model, AppError> {
        self.workflow.upsert_workflow_run(input).await
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

    pub async fn begin_or_resume_workflow_run_validated(
        &self,
        input: UpsertWorkflowRunInput,
    ) -> Result<workflow_run::Model, AppError> {
        crate::workflow::validate_begin_workflow_run_input(&input)
            .map_err(AppError::InvalidArgument)?;
        self.workflow.resume_or_upsert_workflow_run(input).await
    }

    pub(crate) async fn finalize_workflow_result<T>(
        &self,
        workflow_run_id: i64,
        result: Result<T, AppError>,
        context: &str,
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
            (Ok(_), Err(status_error)) => Err(AppError::Internal(anyhow::anyhow!(
                "{context}已完成，但工作流终态写入失败: {status_error}"
            ))),
            (Err(primary), Err(status_error)) => Err(AppError::Internal(anyhow::anyhow!(
                "{context}失败: {primary}；工作流终态写入失败: {status_error}"
            ))),
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
            | crate::workflow::WorkflowRunFinalizeOutcome::Cancelled => Some(
                error_message
                    .map(str::trim)
                    .filter(|msg| !msg.is_empty())
                    .ok_or_else(|| {
                        AppError::InvalidArgument(format!(
                            "workflow {status} 终态必须提供 error_message"
                        ))
                    })?,
            ),
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
        error_message: &str,
    ) -> Result<workflow_run::Model, AppError> {
        self.finalize_workflow_run_outcome_validated(
            workflow_run_id,
            crate::workflow::WorkflowRunFinalizeOutcome::Failed,
            Some(error_message),
        )
        .await
    }

    /// 将 workflow 标为 cancelled（可 resume）。
    pub async fn cancel_workflow_run_status_validated(
        &self,
        workflow_run_id: i64,
        error_message: &str,
    ) -> Result<workflow_run::Model, AppError> {
        self.finalize_workflow_run_outcome_validated(
            workflow_run_id,
            crate::workflow::WorkflowRunFinalizeOutcome::Cancelled,
            Some(error_message),
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

    /// 恢复 workflow 并立即绑定新 generation task，避免异步 worker 启动前暴露旧终态。
    pub async fn resume_workflow_run_with_active_task_validated(
        &self,
        workflow_run_id: i64,
        active_task_id: &str,
    ) -> Result<workflow_run::Model, AppError> {
        crate::workflow::require_positive_workflow_run_id(workflow_run_id)
            .map_err(AppError::InvalidArgument)?;
        self.workflow
            .resume_workflow_run_with_active_task(workflow_run_id, active_task_id)
            .await
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

fn task_info_summary(
    info: &a4n_queue::priority::TaskInfo,
) -> a4n_api_contract::task::TaskInfoSummary {
    a4n_api_contract::task::TaskInfoSummary {
        id: info.id.clone(),
        task_type: info.task_type.as_str().to_string(),
        priority: info.priority.as_str().to_string(),
        status: info.status.as_str().to_string(),
        root_project_id: info.root_project_id,
        source_snapshot_id: info.source_snapshot_id,
        task_scope: info.task_scope.clone(),
        progress: info.progress.as_ref().map(|progress| {
            a4n_api_contract::task::TaskProgressSummary {
                current: progress.current,
                total: progress.total,
                message: progress.message.clone(),
                meta: progress.meta.clone(),
            }
        }),
        created_at: info.created_at.to_string(),
        started_at: info.started_at.map(|value| value.to_string()),
        completed_at: info.completed_at.map(|value| value.to_string()),
    }
}

fn workflow_run_detail_to_api(
    detail: crate::workflow::WorkflowRunDetailSummary,
) -> a4n_api_contract::workflow::WorkflowRunDetailSummary {
    a4n_api_contract::workflow::WorkflowRunDetailSummary {
        run: workflow_run_summary_to_api(detail.run),
        checkpoints: detail
            .checkpoints
            .into_iter()
            .map(
                |checkpoint| a4n_api_contract::workflow::WorkflowCheckpointSummary {
                    id: checkpoint.id,
                    workflow_run_id: checkpoint.workflow_run_id,
                    step_key: checkpoint.step_key,
                    step_order: checkpoint.step_order,
                    status: checkpoint.status,
                    input_fingerprint_sha256: checkpoint.input_fingerprint_sha256,
                    input_fingerprint_json: checkpoint.input_fingerprint_json,
                    checkpoint_json: checkpoint.checkpoint_json,
                    artifacts_json: checkpoint.artifacts_json,
                    generation_run_ids_json: checkpoint.generation_run_ids_json,
                    error_message: checkpoint.error_message,
                    created_at: checkpoint.created_at,
                    updated_at: checkpoint.updated_at,
                    completed_at: checkpoint.completed_at,
                },
            )
            .collect(),
        generation_runs: detail
            .generation_runs
            .into_iter()
            .map(|run| a4n_api_contract::workflow::GenerationRunSummary {
                id: run.id,
                workflow_run_id: run.workflow_run_id,
                workflow: run.workflow,
                step_key: run.step_key,
                call_key: run.call_key,
                title: run.title,
                model_kind: run.model_kind,
                provider: run.provider,
                model: run.model,
                status: run.status,
                input_fingerprint_sha256: run.input_fingerprint_sha256,
                input_fingerprint_json: run.input_fingerprint_json,
                raw_response_json: run.raw_response_json,
                parsed_output_json: run.parsed_output_json,
                validation_json: run.validation_json,
                artifact_refs_json: run.artifact_refs_json,
                attempt: run.attempt,
                response_summary_json: run.response_summary_json,
                error_message: run.error_message,
                created_at: run.created_at,
                completed_at: run.completed_at,
            })
            .collect(),
    }
}

fn active_task_lookup_record(
    persisted: PersistedTaskInfo,
) -> Result<ActiveTaskLookupRecord, AppError> {
    let task = persisted.task;
    let progress =
        task.progress
            .as_ref()
            .map(|progress| a4n_api_contract::task::TaskProgressSummary {
                current: progress.current,
                total: progress.total,
                message: progress.message.clone(),
                meta: progress.meta.clone(),
            });
    let progress_events = task
        .progress
        .as_ref()
        .map(
            |progress| a4n_api_contract::workflow::WorkflowProgressEvent {
                id: format!("{}-snapshot", task.id),
                current: progress.current,
                total: progress.total,
                message: progress.message.clone(),
                meta: progress.meta.clone(),
                timestamp: progress
                    .updated_at
                    .clone()
                    .unwrap_or_else(|| task.started_at.unwrap_or(task.created_at).to_string()),
            },
        )
        .into_iter()
        .collect();
    Ok(ActiveTaskLookupRecord {
        task_id: task.id.clone(),
        project_id: task.project_id,
        root_project_id: task.root_project_id,
        status: task.status.as_str().to_string(),
        workflow: a4n_api_contract::workflow::WorkflowStateSummary {
            task_id: task.id,
            root_project_id: task.root_project_id,
            source_snapshot_id: task.source_snapshot_id,
            task_scope: task.task_scope,
            progress,
            progress_events,
        },
    })
}

#[async_trait::async_trait]
impl WorkflowTaskLookupApplicationPort for OrchestratorContext {
    async fn task_by_id(&self, task_id: &str) -> Result<Option<TaskLookupRecord>, AppError> {
        Ok(self
            .workflow
            .persisted_task_by_id(task_id)
            .await?
            .map(|persisted| TaskLookupRecord {
                task_id: persisted.task.id.clone(),
                project_id: persisted.task.project_id,
                root_project_id: persisted.task.root_project_id,
                workflow_run_id: persisted.workflow_run_id,
                status: persisted.task.status.as_str().to_string(),
            }))
    }

    async fn active_tasks(&self) -> Result<Vec<ActiveTaskLookupRecord>, AppError> {
        self.workflow.active_tasks().await
    }

    async fn workflow_access_by_active_task_id(
        &self,
        task_id: &str,
    ) -> Result<Option<WorkflowTaskAccessRecord>, AppError> {
        Ok(self
            .workflow
            .workflow_run_access_record_by_active_task_id(task_id)
            .await?
            .map(|record| WorkflowTaskAccessRecord {
                workflow_run_id: record.id,
                project_id: record.project_id,
                root_project_id: record.root_project_id,
            }))
    }

    async fn workflow_run_for_resume(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowResumeRecord, AppError> {
        self.workflow
            .workflow_run_resume_record(workflow_run_id)
            .await
    }

    async fn workflow_run_summary(
        &self,
        workflow_run_id: i64,
    ) -> Result<a4n_api_contract::workflow::WorkflowRunSummary, AppError> {
        self.workflow
            .workflow_run_summary(workflow_run_id)
            .await
            .map(workflow_run_summary_to_api)
    }

    async fn workflow_progress_events(
        &self,
        workflow_run_id: i64,
        limit: usize,
    ) -> Result<Vec<a4n_api_contract::workflow::WorkflowProgressEvent>, AppError> {
        self.workflow
            .workflow_progress_events(workflow_run_id, limit)
            .await
            .map(|events| {
                events
                    .into_iter()
                    .map(|event| a4n_api_contract::workflow::WorkflowProgressEvent {
                        id: event.id,
                        current: event.current,
                        total: event.total,
                        message: event.message,
                        meta: event.meta,
                        timestamp: event.timestamp,
                    })
                    .collect()
            })
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::workflow::WorkflowReadApplicationPort for OrchestratorContext {
    async fn list_for_user(
        &self,
        user_id: i64,
        input: a4n_api_contract::workflow::ListWorkflowRunsInput,
    ) -> Result<Vec<a4n_api_contract::workflow::WorkflowRunSummary>, AppError> {
        crate::workflow::validate_optional_project_id(input.project_id)
            .map_err(AppError::InvalidArgument)?;
        if !(1..=200).contains(&input.limit) {
            return Err(AppError::InvalidArgument(
                "limit 必须在 1 到 200 之间".to_string(),
            ));
        }
        let project_scoped = input.project_id.is_some();
        if let Some(project_id) = input.project_id {
            self.verify_project_permission(
                project_id,
                user_id,
                crate::collaboration::ProjectAccessRole::Editor,
            )
            .await?;
        }
        let runs = self
            .workflow_runs(crate::workflow::ListWorkflowRunsInput {
                project_id: input.project_id,
                chapter_id: input.chapter_id,
                chapter_ids: None,
                workflow_kind: input.workflow_kind,
                status: input.status,
                resumable_only: Some(input.resumable_only),
                limit: Some(input.limit),
            })
            .await?;
        if project_scoped {
            return Ok(runs.into_iter().map(workflow_run_summary_to_api).collect());
        }

        let project_ids = runs
            .iter()
            .filter_map(|run| run.project_id.or(run.root_project_id))
            .collect::<Vec<_>>();
        let visible_project_ids = if project_ids.is_empty() {
            HashSet::new()
        } else {
            self.project_ids_with_permission(
                &project_ids,
                user_id,
                crate::collaboration::ProjectAccessRole::Editor,
            )
            .await?
            .into_iter()
            .collect()
        };
        let unscoped_visible = runs
            .iter()
            .any(|run| run.project_id.is_none() && run.root_project_id.is_none())
            && self.is_active_admin_user(user_id).await?;
        Ok(runs
            .into_iter()
            .filter(|run| match run.project_id.or(run.root_project_id) {
                Some(project_id) => visible_project_ids.contains(&project_id),
                None => unscoped_visible,
            })
            .map(workflow_run_summary_to_api)
            .collect())
    }

    async fn detail_for_user(
        &self,
        user_id: i64,
        input: a4n_api_contract::workflow::GetWorkflowRunInput,
    ) -> Result<a4n_api_contract::workflow::WorkflowRunDetailSummary, AppError> {
        crate::workflow::require_positive_workflow_run_id(input.workflow_run_id)
            .map_err(AppError::InvalidArgument)?;
        let detail = self
            .workflow_run_detail(crate::workflow::GetWorkflowRunInput {
                workflow_run_id: input.workflow_run_id,
                include_generation_runs: input.include_generation_runs,
            })
            .await?;
        match detail.run.project_id.or(detail.run.root_project_id) {
            Some(project_id) => {
                self.verify_project_permission(
                    project_id,
                    user_id,
                    crate::collaboration::ProjectAccessRole::Editor,
                )
                .await?;
            }
            None if !self.is_active_admin_user(user_id).await? => {
                return Err(AppError::Forbidden(
                    "无权查看未绑定项目的 workflow".to_string(),
                ));
            }
            None => {}
        }
        Ok(workflow_run_detail_to_api(detail))
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::task::TaskReadApplicationPort for OrchestratorContext {
    async fn history(
        &self,
        project_id: Option<i64>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<a4n_api_contract::task::TaskInfoSummary>, AppError> {
        self.workflow.task_history(project_id, offset, limit).await
    }
}

#[async_trait::async_trait]
impl WorkflowCancellationPort for OrchestratorContext {
    async fn mark_running(&self, workflow_run_id: i64) -> Result<(), AppError> {
        self.mark_workflow_running_validated(workflow_run_id)
            .await
            .map(|_| ())
    }

    async fn cancel(&self, workflow_run_id: i64, reason: &str) -> Result<(), AppError> {
        self.cancel_workflow_run_and_interrupt_videos(workflow_run_id, reason)
            .await
            .map(|_| ())
    }

    async fn resume_with_active_task(
        &self,
        workflow_run_id: i64,
        active_task_id: &str,
    ) -> Result<(), AppError> {
        self.resume_workflow_run_with_active_task_validated(workflow_run_id, active_task_id)
            .await
            .map(|_| ())
    }
}

#[cfg(test)]
mod task_read_tests {
    use super::task_info_summary;
    use a4n_domain::progress::{ProgressMeta, ProgressPhaseKind};
    use a4n_queue::priority::{TaskInfo, TaskKind, TaskPriority, TaskProgress, TaskStatus};
    use chrono::NaiveDate;

    #[test]
    fn task_history_projection_stays_in_the_workflow_owner() {
        let created_at = NaiveDate::from_ymd_opt(2026, 9, 10)
            .unwrap()
            .and_hms_opt(1, 2, 3)
            .unwrap();
        let started_at = NaiveDate::from_ymd_opt(2026, 9, 10)
            .unwrap()
            .and_hms_opt(1, 2, 4)
            .unwrap();
        let task = TaskInfo {
            id: "task-1".to_string(),
            task_type: TaskKind::MangaGeneration,
            priority: TaskPriority::Medium,
            status: TaskStatus::Running,
            project_id: Some(11),
            root_project_id: Some(7),
            source_snapshot_id: Some(13),
            task_scope: Some("chapter".to_string()),
            progress: Some(TaskProgress {
                current: 2,
                total: 4,
                message: "drawing".to_string(),
                meta: Some(ProgressMeta::new(
                    "manga_generation",
                    "draw",
                    "draw",
                    ProgressPhaseKind::AssetGeneration,
                )),
                updated_at: None,
            }),
            created_at,
            started_at: Some(started_at),
            completed_at: None,
        };

        let summary = task_info_summary(&task);

        assert_eq!(summary.id, "task-1");
        assert_eq!(summary.task_type, "manga_generation");
        assert_eq!(summary.priority, "medium");
        assert_eq!(summary.status, "running");
        assert_eq!(summary.root_project_id, Some(7));
        assert_eq!(summary.source_snapshot_id, Some(13));
        assert_eq!(summary.task_scope.as_deref(), Some("chapter"));
        assert_eq!(summary.created_at, created_at.to_string());
        let started_at_text = started_at.to_string();
        assert_eq!(
            summary.started_at.as_deref(),
            Some(started_at_text.as_str())
        );
        let progress = summary.progress.expect("progress should be projected");
        assert_eq!(progress.current, 2);
        assert_eq!(progress.total, 4);
        assert_eq!(progress.message, "drawing");
        assert_eq!(progress.meta.unwrap().stage_code, "draw");
    }
}
