//! workflow facade。

use super::*;

impl OrchestratorContext {
    pub(crate) async fn workflow_run_summary(
        &self,
        workflow_run_id: i64,
    ) -> Result<WorkflowRunSummary, AppError> {
        self.workflow.workflow_run_summary(workflow_run_id).await
    }

    pub(crate) async fn workflow_run_models(
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

    pub(crate) async fn workflow_runs_by_project_kind(
        &self,
        project_id: i64,
        workflow_kind: &str,
    ) -> Result<Vec<WorkflowRunResumeRecord>, AppError> {
        self.workflow
            .workflow_runs_by_project_kind(project_id, workflow_kind)
            .await
    }

    pub(crate) async fn latest_workflow_runs_for_chapters(
        &self,
        project_id: i64,
        workflow_kind: &str,
        chapter_ids: &[i64],
    ) -> Result<Vec<workflow_run::Model>, AppError> {
        self.workflow
            .latest_workflow_runs_for_chapters(project_id, workflow_kind, chapter_ids)
            .await
    }

    pub(crate) async fn workflow_run_model_by_id(
        &self,
        workflow_run_id: i64,
    ) -> Result<workflow_run::Model, AppError> {
        self.workflow
            .workflow_run_model_by_id(workflow_run_id)
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

    pub(crate) async fn workflow_run_statuses_by_ids(
        &self,
        workflow_run_ids: &[i64],
    ) -> Result<Vec<(i64, String)>, AppError> {
        self.workflow
            .workflow_run_statuses_by_ids(workflow_run_ids)
            .await
    }

    pub(crate) async fn latest_workflow_run_by_business_key(
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

    pub(crate) async fn workflow_checkpoint_models(
        &self,
        workflow_run_id: i64,
    ) -> Result<Vec<workflow_checkpoint::Model>, AppError> {
        self.workflow
            .workflow_checkpoint_models(workflow_run_id)
            .await
    }

    pub(crate) async fn workflow_checkpoint_summaries(
        &self,
        workflow_run_id: i64,
    ) -> Result<Vec<WorkflowCheckpointSummary>, AppError> {
        self.workflow
            .workflow_checkpoint_summaries(workflow_run_id)
            .await
    }

    pub(crate) async fn workflow_checkpoint_models_for_runs(
        &self,
        workflow_run_ids: &[i64],
    ) -> Result<Vec<workflow_checkpoint::Model>, AppError> {
        self.workflow
            .workflow_checkpoint_models_for_runs(workflow_run_ids)
            .await
    }

    pub(crate) async fn completed_workflow_checkpoint_by_fingerprint(
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
    pub(crate) async fn load_completed_llm_call_checkpoint(
        &self,
        workflow_run_id: i64,
        call_key: &str,
        input_fingerprint_json: &serde_json::Value,
    ) -> Result<Option<CompletedLlmCallCheckpoint>, AppError> {
        let checkpoint = self
            .workflow
            .load_completed_llm_call_checkpoint(workflow_run_id, call_key, input_fingerprint_json)
            .await?;
        if checkpoint.is_some() {
            return Ok(checkpoint);
        }
        self.load_timing_upgraded_checkpoint(workflow_run_id, call_key, input_fingerprint_json)
            .await
    }

    pub(crate) async fn validate_llm_call_output<T>(
        &self,
        workflow_run_id: i64,
        call_key: &str,
        input_fingerprint_json: &serde_json::Value,
        output: Result<T, AppError>,
    ) -> Result<T, AppError> {
        self.workflow
            .validate_llm_call_output(workflow_run_id, call_key, input_fingerprint_json, output)
            .await
    }

    pub(crate) async fn cancel_workflow_run_and_interrupt_videos(
        &self,
        workflow_run_id: i64,
        reason: &str,
    ) -> Result<CancelWorkflowRunPersistenceResult, AppError> {
        let cancellation = self
            .workflow
            .cancel_workflow_run_persistence(workflow_run_id, reason)
            .await?;
        self.apply_workflow_cancellation_side_effects(
            WorkflowCancellationResult {
                workflow_run_id: cancellation.workflow_run_id,
                active_task_id: cancellation.active_task_id.clone(),
                interrupted_generated_video_ids: cancellation
                    .interrupted_generated_video_ids
                    .clone(),
                already_terminal: cancellation.already_terminal,
                cancelled: cancellation.cancelled,
            },
            reason,
        )
        .await?;
        Ok(cancellation)
    }

    pub(crate) async fn apply_workflow_cancellation_side_effects(
        &self,
        cancellation: WorkflowCancellationResult,
        reason: &str,
    ) -> Result<(), AppError> {
        if cancellation.cancelled {
            self.task_queue
                .cancel_workflow_llm_calls(cancellation.workflow_run_id)
                .await?;
            if let Some(task_id) = cancellation.active_task_id.as_deref() {
                self.cancel_task_handle(task_id);
                if let Err(error) = self.queue_cancel(task_id).await {
                    tracing::warn!(
                        workflow_run_id = cancellation.workflow_run_id,
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
        Ok(())
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
        let mut cancelled_video_ids = Vec::new();
        for video_id in video_ids {
            let Some(video) = videos_by_id.get(video_id) else {
                tracing::warn!(video_id, "取消工作流时读取关联视频失败: 视频不存在");
                continue;
            };
            let already_cancelled =
                video.status == a4n_domain::video_workflow::VIDEO_STATUS_CANCELLED;
            let final_status = is_video_status_final_for_cancel_interrupt(&video.status);
            let next_status =
                if crate::video::normalized_provider_job_id(video.provider_job_id.as_deref())
                    .is_some()
                {
                    a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED
                } else {
                    a4n_domain::video_workflow::VIDEO_STATUS_CANCELLED
                };
            if already_cancelled
                || (!final_status
                    && next_status == a4n_domain::video_workflow::VIDEO_STATUS_CANCELLED)
            {
                cancelled_video_ids.push(video.id);
            }
            if final_status {
                continue;
            }
            inputs.push(GeneratedVideoErrorStatusBatchInput {
                video_id: video.id,
                prompt_id: a4n_domain::video::generated_video_prompt_id(
                    video.metadata_json.as_ref(),
                ),
                status: next_status.to_string(),
            });
        }
        if !inputs.is_empty() {
            let missing = self
                .video
                .mark_generated_videos_error_status_and_prompt_records(&inputs, reason)
                .await?;
            for video_id in missing {
                tracing::warn!(video_id, "取消工作流时读取关联视频失败: 视频不存在");
            }
        }
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
        for video_id in &cancelled_video_ids {
            crate::video::ensure_provider_generation_run_updated(
                *video_id,
                cancelled_run_counts
                    .get(video_id)
                    .copied()
                    .unwrap_or_default(),
                "取消 provider 视频生成",
            )
            .map_err(AppError::InvalidArgument)?;
        }
        Ok(())
    }

    pub(crate) async fn upsert_workflow_run(
        &self,
        input: UpsertWorkflowRunInput,
    ) -> Result<workflow_run::Model, AppError> {
        self.workflow.upsert_workflow_run(input).await
    }

    pub(crate) async fn claim_exclusive_workflow_run(
        &self,
        input: ClaimExclusiveWorkflowRunInput,
    ) -> Result<workflow_run::Model, AppError> {
        let run = self.workflow.claim_exclusive_workflow_run(input).await?;
        self.generation
            .refresh_session_after_workflow_binding(run.active_task_id.as_deref())
            .await
            .map_err(|error| AppError::InvalidState {
                current: "generation_session_workflow_binding".to_string(),
                operation: error.to_string(),
            })?;
        Ok(run)
    }

    pub(crate) async fn mark_workflow_status(
        &self,
        workflow_run_id: i64,
        status: &str,
        can_resume: bool,
        error_message: Option<&str>,
    ) -> Result<workflow_run::Model, AppError> {
        let run = self
            .workflow
            .mark_workflow_status(workflow_run_id, status, can_resume, error_message)
            .await?;
        if status == a4n_domain::workflow::WORKFLOW_STATUS_CANCELLED {
            self.task_queue
                .cancel_workflow_llm_calls(workflow_run_id)
                .await?;
        }
        Ok(run)
    }

    /// 开始工作流：校验 business_key/kind/id 与 RUNNING 后 upsert。
    /// Host 仍可构造指纹与业务字段，但状态推进 owner 在 orch。
    pub(crate) async fn begin_workflow_run_validated(
        &self,
        input: UpsertWorkflowRunInput,
    ) -> Result<workflow_run::Model, AppError> {
        crate::workflow::validate_begin_workflow_run_input(&input)
            .map_err(AppError::InvalidArgument)?;
        self.upsert_workflow_run(input).await
    }

    pub(crate) async fn begin_or_resume_workflow_run_validated(
        &self,
        input: UpsertWorkflowRunInput,
    ) -> Result<workflow_run::Model, AppError> {
        crate::workflow::validate_begin_workflow_run_input(&input)
            .map_err(AppError::InvalidArgument)?;
        self.workflow.resume_or_upsert_workflow_run(input).await
    }

    pub(crate) async fn continue_completed_workflow_run_validated(
        &self,
        source_run_id: i64,
        input: UpsertWorkflowRunInput,
    ) -> Result<workflow_run::Model, AppError> {
        crate::workflow::validate_begin_workflow_run_input(&input)
            .map_err(AppError::InvalidArgument)?;
        self.workflow
            .continue_completed_workflow_run(source_run_id, input)
            .await
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
    pub(crate) async fn finalize_workflow_run_outcome_validated(
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
    pub(crate) async fn complete_workflow_run_validated(
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
    pub(crate) async fn fail_workflow_run_validated(
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
    pub(crate) async fn cancel_workflow_run_status_validated(
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
    pub(crate) async fn mark_workflow_running_validated(
        &self,
        workflow_run_id: i64,
    ) -> Result<workflow_run::Model, AppError> {
        crate::workflow::require_positive_workflow_run_id(workflow_run_id)
            .map_err(AppError::InvalidArgument)?;
        self.workflow.resume_workflow_run(workflow_run_id).await
    }

    /// 恢复 workflow 并立即绑定新 generation task，避免异步 worker 启动前暴露旧终态。
    pub(crate) async fn resume_workflow_run_with_active_task_validated(
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

    pub(crate) async fn mark_workflow_current_step(
        &self,
        workflow_run_id: i64,
        current_step_key: &str,
    ) -> Result<workflow_run::Model, AppError> {
        self.workflow
            .mark_workflow_current_step(workflow_run_id, current_step_key)
            .await
    }

    pub(crate) async fn upsert_workflow_checkpoint(
        &self,
        input: UpsertWorkflowCheckpointInput,
    ) -> Result<workflow_checkpoint::Model, AppError> {
        self.workflow.upsert_workflow_checkpoint(input).await
    }

    pub(crate) async fn record_workflow_progress_event(
        &self,
        input: WorkflowProgressEventInput<'_>,
    ) -> Result<workflow_progress_event::Model, AppError> {
        self.workflow.record_workflow_progress_event(input).await
    }

    pub(crate) async fn persist_workflow_progress_snapshot(
        &self,
        workflow_run_id: i64,
        snapshot: serde_json::Value,
    ) -> Result<workflow_run::Model, AppError> {
        self.workflow
            .persist_workflow_progress_snapshot(workflow_run_id, snapshot)
            .await
    }

    /// 保存已完成的 LLM call checkpoint。
    pub(crate) async fn save_completed_llm_call_checkpoint(
        &self,
        input: CompletedLlmCallCheckpointInput,
    ) -> Result<(), AppError> {
        self.workflow
            .save_completed_llm_call_checkpoint(input)
            .await
    }

    pub(crate) async fn delete_workflow_checkpoints_by_keys(
        &self,
        workflow_run_id: i64,
        step_keys: &[&str],
    ) -> Result<u64, AppError> {
        self.workflow
            .delete_workflow_checkpoints_by_keys(workflow_run_id, step_keys)
            .await
    }
}
