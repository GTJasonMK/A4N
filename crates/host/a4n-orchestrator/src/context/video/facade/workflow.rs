//! 视频 workflow facade。

use super::*;

impl OrchestratorContext {
    /// 章节视频提示词生成：校验后创建 workflow_run（RUNNING）。
    /// Host 仍负责 LLM 分阶段与提示词落库。
    pub(crate) async fn begin_video_prompt_workflow_validated(
        &self,
        workflow_kind: &str,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        workflow_step_key: &str,
        source: &crate::video::ChapterVideoSource,
        options: &crate::video::ChapterVideoPromptOptions,
        active_task_id: &str,
        root_project_id: i64,
        retry_of_task_id: Option<&str>,
    ) -> Result<a4n_domain::workflow::WorkflowRunSummary, AppError> {
        let (input, _fingerprint) = crate::video::video_prompt_workflow_run_input(
            workflow_kind,
            project_id,
            chapter_id,
            workflow_mode,
            workflow_step_key,
            source,
            options,
            active_task_id,
            root_project_id,
        )
        .map_err(AppError::InvalidArgument)?;
        self.claim_exclusive_workflow_run(crate::workflow::ClaimExclusiveWorkflowRunInput {
            run: input,
            resume_from_task_id: retry_of_task_id.map(str::to_owned),
        })
        .await
    }

    /// 视频提示词优化：校验后创建 workflow_run。
    pub(crate) async fn begin_video_prompt_optimization_workflow_validated(
        &self,
        project_id: i64,
        prompt_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        current_step_key: &str,
        input_fingerprint_json: serde_json::Value,
    ) -> Result<a4n_domain::workflow::WorkflowRunSummary, AppError> {
        let input = crate::video::video_prompt_optimization_workflow_run_input(
            project_id,
            prompt_id,
            chapter_id,
            workflow_mode,
            current_step_key,
            input_fingerprint_json,
        )
        .map_err(AppError::InvalidArgument)?;
        self.begin_or_resume_workflow_run_validated(input).await
    }

    /// 视频 provider 进度：校验后创建/恢复 RUNNING workflow_run。
    pub(crate) async fn begin_video_provider_progress_workflow_validated(
        &self,
        video: &GeneratedVideoRecord,
        job_id: &str,
    ) -> Result<i64, AppError> {
        let business_key = crate::video::video_provider_workflow_business_key(video.id)
            .map_err(AppError::InvalidArgument)?;
        if let Some(run) = self
            .latest_workflow_run_status_record_by_business_key(&business_key)
            .await?
        {
            let run = self
                .mark_workflow_status(run.id, crate::workflow::WORKFLOW_STATUS_RUNNING, true, None)
                .await?;
            return Ok(run.id);
        }
        let input = crate::video::video_provider_progress_workflow_run_input(video, job_id)
            .map_err(AppError::InvalidArgument)?;
        let run = self.begin_workflow_run_validated(input).await?;
        Ok(run.id)
    }

    /// provider 轮询进度及 Canvas 节点投影的唯一写入入口。
    pub(crate) async fn record_video_provider_progress_validated(
        &self,
        video: &GeneratedVideoRecord,
        current: u32,
        total: u32,
        message: &str,
    ) -> Result<i64, AppError> {
        if a4n_domain::video_workflow::is_terminal_video_status(&video.status) {
            return Err(AppError::InvalidState {
                current: video.status.clone(),
                operation: "记录已终态视频 provider 进度".to_string(),
            });
        }
        let job_id = crate::video::normalized_provider_job_id(video.provider_job_id.as_deref())
            .ok_or_else(|| AppError::InvalidArgument("视频缺少 provider_job_id".to_string()))?;
        let run_id = self
            .begin_video_provider_progress_workflow_validated(video, job_id)
            .await?;
        self.record_workflow_progress_event(WorkflowProgressEventInput {
            workflow_run_id: run_id,
            task_id: None,
            current,
            total,
            message,
            meta: None,
        })
        .await?;

        let Some(canvas) = video
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("canvas"))
        else {
            return Ok(run_id);
        };
        let workflow_run_id = canvas
            .get("workflow_run_id")
            .and_then(Value::as_i64)
            .filter(|id| *id > 0)
            .ok_or_else(|| {
                AppError::InvalidArgument("画布视频缺少有效 workflow_run_id".to_string())
            })?;
        let graph_id = canvas
            .get("graph_id")
            .and_then(Value::as_i64)
            .filter(|id| *id > 0)
            .ok_or_else(|| AppError::InvalidArgument("画布视频缺少有效 graph_id".to_string()))?;
        let node_id = canvas
            .get("node_id")
            .and_then(Value::as_i64)
            .filter(|id| *id > 0)
            .ok_or_else(|| AppError::InvalidArgument("画布视频缺少有效 node_id".to_string()))?;
        let step_key =
            crate::canvas::canvas_node_step_key(node_id).map_err(AppError::InvalidArgument)?;
        self.mark_workflow_current_step(workflow_run_id, &step_key)
            .await?;
        self.upsert_workflow_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id,
            step_key,
            step_order: 0,
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: Some(json!({
                "generated_video_id": video.id,
                "provider_job_id": video.provider_job_id,
                "graph_id": graph_id,
                "node_id": node_id,
            })),
            checkpoint_json: None,
            artifacts_json: Some(json!({
                "data_type": "video",
                "generated_video_id": video.id,
                "status": video.status,
                "provider_job_id": video.provider_job_id,
            })),
            generation_run_ids_json: None,
            error_message: None,
            completed: false,
        })
        .await?;
        self.record_workflow_progress_event(WorkflowProgressEventInput {
            workflow_run_id,
            task_id: None,
            current,
            total,
            message,
            meta: None,
        })
        .await?;
        Ok(run_id)
    }

    /// 视频 provider workflow：统一写入 run、checkpoint 与 generation_run 关联。
    pub(crate) async fn mark_video_provider_workflow_validated(
        &self,
        video: &GeneratedVideoRecord,
        status: &str,
        can_resume: bool,
        error_message: Option<&str>,
    ) -> Result<(), AppError> {
        let Some(job_id) =
            crate::video::normalized_provider_job_id(video.provider_job_id.as_deref())
        else {
            return Ok(());
        };
        let fingerprint = crate::video::video_provider_workflow_fingerprint(video, job_id);
        let fingerprint_sha = source_content_sha256(&serde_json::to_string(&fingerprint)?);
        let artifacts = json!({
            "targets": [{
                "target_table": "generated_video",
                "target_id": video.id,
                "target_kind": "video_provider_job",
                "role": "provider_job",
                "provider_job_id": job_id,
            }],
        });
        let business_key = crate::video::video_provider_workflow_business_key(video.id)
            .map_err(AppError::InvalidArgument)?;
        let run = match self
            .latest_workflow_run_status_record_by_business_key(&business_key)
            .await?
        {
            Some(run)
                if run.status == status
                    && matches!(
                        status,
                        crate::workflow::WORKFLOW_STATUS_COMPLETED
                            | crate::workflow::WORKFLOW_STATUS_FAILED
                            | crate::workflow::WORKFLOW_STATUS_CANCELLED
                            | crate::workflow::WORKFLOW_STATUS_SUPERSEDED
                    ) =>
            {
                return Ok(());
            }
            Some(run) => run.id,
            None => {
                let input = crate::video::video_provider_progress_workflow_run_input(video, job_id)
                    .map_err(AppError::InvalidArgument)?;
                self.begin_workflow_run_validated(input).await?.id
            }
        };
        self.upsert_workflow_checkpoint(a4n_db::workflow::UpsertWorkflowCheckpointInput {
            workflow_run_id: run,
            step_key: "provider_job".to_string(),
            step_order: 1,
            status: status.to_string(),
            input_fingerprint_sha256: Some(fingerprint_sha.clone()),
            input_fingerprint_json: Some(fingerprint.clone()),
            checkpoint_json: Some(json!({
                "provider_job_id": job_id,
                "generated_video_id": video.id,
                "status": video.status,
            })),
            artifacts_json: Some(artifacts.clone()),
            generation_run_ids_json: None,
            error_message: error_message.map(str::to_string),
            completed: matches!(
                status,
                crate::workflow::WORKFLOW_STATUS_COMPLETED
                    | crate::workflow::WORKFLOW_STATUS_FAILED
            ),
        })
        .await?;
        self.attach_generation_runs_to_workflow_for_target(
            a4n_db::generation::AttachGenerationRunsToWorkflowInput {
                target_table: "generated_video".to_string(),
                target_id: video.id,
                workflow_run_id: run,
                call_key: Some("provider_job".to_string()),
                input_fingerprint_sha256: Some(fingerprint_sha),
                input_fingerprint_json: Some(fingerprint),
                artifact_refs_json: Some(artifacts),
            },
        )
        .await?;
        self.mark_workflow_status(run, status, can_resume, error_message)
            .await?;
        Ok(())
    }
}
