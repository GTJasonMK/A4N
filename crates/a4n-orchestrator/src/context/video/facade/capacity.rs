//! 视频并发容量与中断回收 facade。

use super::*;

impl OrchestratorContext {
    pub(crate) async fn ensure_video_capacity(
        &self,
        config_id: i64,
        max_concurrent: i32,
    ) -> Result<(), AppError> {
        validate_video_max_concurrent(max_concurrent).map_err(AppError::InvalidArgument)?;
        let limit = max_concurrent as usize;
        let used = self
            .active_video_ids_for_config_reconciled(config_id)
            .await?
            .len();
        if used >= limit {
            return Err(AppError::InvalidArgument(format!(
                "当前视频配置已有 {used} 个进行中的任务，已达到并发上限 {limit}"
            )));
        }
        Ok(())
    }

    pub(crate) async fn ensure_created_video_capacity(
        &self,
        config_id: i64,
        max_concurrent: i32,
        video_id: i64,
    ) -> Result<(), AppError> {
        validate_video_max_concurrent(max_concurrent).map_err(AppError::InvalidArgument)?;
        let limit = max_concurrent as usize;
        let active = self
            .active_video_ids_for_config_reconciled(config_id)
            .await?;
        if active.len() <= limit || active.iter().take(limit).any(|id| *id == video_id) {
            return Ok(());
        }
        let existing_active_count = active.iter().filter(|id| **id != video_id).count();
        Err(AppError::InvalidArgument(format!(
            "当前视频配置已有 {existing_active_count} 个进行中的任务，已达到并发上限 {limit}"
        )))
    }

    async fn active_video_ids_for_config_reconciled(
        &self,
        config_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        let active_ids = self
            .active_generated_video_ids_for_config(config_id)
            .await?;
        let active_videos = self.video.generated_videos_by_ids(&active_ids).await?;
        let videos_by_id = active_videos
            .into_iter()
            .map(|video| (video.id, video))
            .collect::<std::collections::HashMap<_, _>>();
        let provider_business_keys = videos_by_id
            .values()
            .filter(|video| {
                video
                    .provider_job_id
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|job_id| !job_id.is_empty())
            })
            .map(|video| format!("video_provider_job:{}", video.id))
            .collect::<Vec<_>>();
        let canvas_workflow_run_ids = videos_by_id
            .values()
            .filter_map(|video| {
                video
                    .metadata_json
                    .as_ref()
                    .and_then(|metadata| metadata.get("canvas"))
                    .and_then(|canvas| canvas.get("workflow_run_id"))
                    .and_then(serde_json::Value::as_i64)
            })
            .collect::<std::collections::HashSet<_>>();
        let canvas_workflow_run_id_list =
            canvas_workflow_run_ids.iter().copied().collect::<Vec<_>>();
        let (provider_workflow_runs_result, canvas_workflow_runs_result) = futures::join!(
            self.latest_workflow_run_recovery_records_by_business_keys(&provider_business_keys),
            self.workflow_run_statuses_by_ids(&canvas_workflow_run_id_list),
        );
        let provider_workflow_runs = provider_workflow_runs_result?;
        let provider_workflow_runs_by_key = provider_workflow_runs
            .into_iter()
            .map(|run| (run.business_key.clone(), run))
            .collect::<std::collections::HashMap<_, _>>();
        let canvas_workflow_runs_by_id = canvas_workflow_runs_result?
            .into_iter()
            .map(|run| (run.0, run))
            .collect::<std::collections::HashMap<_, _>>();
        let mut retained = Vec::with_capacity(active_ids.len());
        for video_id in active_ids {
            let Some(video) = videos_by_id.get(&video_id) else {
                continue;
            };
            let provider_workflow_run =
                provider_workflow_runs_by_key.get(&format!("video_provider_job:{video_id}"));
            let canvas_workflow_run = video
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("canvas"))
                .and_then(|canvas| canvas.get("workflow_run_id"))
                .and_then(serde_json::Value::as_i64)
                .and_then(|workflow_run_id| canvas_workflow_runs_by_id.get(&workflow_run_id));
            if self
                .reconcile_interrupted_provider_video(
                    video,
                    provider_workflow_run,
                    canvas_workflow_run,
                )
                .await?
            {
                continue;
            }
            retained.push(video_id);
        }
        Ok(retained)
    }

    async fn reconcile_interrupted_provider_video(
        &self,
        video: &generated_video::Model,
        provider_workflow_run: Option<&WorkflowRunRecoveryRecord>,
        canvas_workflow_run: Option<&(i64, String)>,
    ) -> Result<bool, AppError> {
        if video
            .provider_job_id
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
        {
            return Ok(false);
        }
        if self
            .reconcile_terminal_canvas_workflow_video(video, canvas_workflow_run)
            .await?
        {
            return Ok(true);
        }
        let Some(run) = provider_workflow_run else {
            return Ok(false);
        };
        if run.status != WORKFLOW_STATUS_INTERRUPTED {
            return Ok(false);
        }
        let message = run
            .error_message
            .as_deref()
            .unwrap_or("视频 provider 轮询已中断，可稍后恢复轮询");
        self.mark_generated_video_interrupted_validated(
            video.id,
            a4n_domain::video::generated_video_prompt_id(video.metadata_json.as_ref()),
            message,
        )
        .await?;
        Ok(true)
    }

    async fn reconcile_terminal_canvas_workflow_video(
        &self,
        video: &generated_video::Model,
        canvas_workflow_run: Option<&(i64, String)>,
    ) -> Result<bool, AppError> {
        let Some(_workflow_run_id) = video
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("canvas"))
            .and_then(|canvas| canvas.get("workflow_run_id"))
            .and_then(serde_json::Value::as_i64)
        else {
            return Ok(false);
        };
        let Some(run) = canvas_workflow_run else {
            return Ok(false);
        };
        if !matches!(
            run.1.as_str(),
            WORKFLOW_STATUS_CANCELLED | WORKFLOW_STATUS_FAILED | WORKFLOW_STATUS_SUPERSEDED
        ) {
            return Ok(false);
        }
        let message = format!(
            "所属画布运行 #{} 已{}",
            run.0,
            terminal_workflow_status_label(&run.1)
        );
        self.mark_generated_video_interrupted_validated(
            video.id,
            a4n_domain::video::generated_video_prompt_id(video.metadata_json.as_ref()),
            &message,
        )
        .await?;
        Ok(true)
    }
}
