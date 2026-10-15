//! 视频记录生命周期 facade。

use super::*;

impl OrchestratorContext {
    pub(crate) async fn create_video_record(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
    ) -> Result<GeneratedVideoRecord, AppError> {
        self.video.create_generated_video_record(input).await
    }

    /// 内部写路径：无 project_id 门禁；对外请用 `*_validated`。
    pub(crate) async fn create_video_record_with_generation_run(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
        generation_run: RecordStartedGenerationRunInput,
    ) -> Result<GeneratedVideoRecord, AppError> {
        self.video
            .create_generated_video_record_with_generation_run(input, generation_run)
            .await
    }

    /// 创建视频记录并挂 generation_run：校验 project_id。
    pub(crate) async fn create_video_record_with_generation_run_validated(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
        generation_run: RecordStartedGenerationRunInput,
    ) -> Result<GeneratedVideoRecord, AppError> {
        crate::video::ensure_positive_project_id_for_video(input.project_id)
            .map_err(AppError::InvalidArgument)?;
        self.create_video_record_with_generation_run(input, generation_run)
            .await
    }

    /// 内部写路径：无 project/prompt 门禁；对外请用 `*_validated`。
    pub(crate) async fn create_prompt_video_record_with_generation_run(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
        generation_run: RecordStartedGenerationRunInput,
        prompt_id: i64,
    ) -> Result<GeneratedVideoRecord, AppError> {
        self.video
            .create_prompt_generated_video_record_with_generation_run(
                input,
                generation_run,
                prompt_id,
            )
            .await
    }

    /// 章节提示词视频创建：校验 project_id 与 prompt_id。
    pub(crate) async fn create_prompt_video_record_with_generation_run_validated(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
        generation_run: RecordStartedGenerationRunInput,
        prompt_id: i64,
    ) -> Result<GeneratedVideoRecord, AppError> {
        crate::video::ensure_positive_project_id_for_video(input.project_id)
            .map_err(AppError::InvalidArgument)?;
        crate::video::ensure_positive_video_prompt_id(prompt_id)
            .map_err(AppError::InvalidArgument)?;
        self.create_prompt_video_record_with_generation_run(input, generation_run, prompt_id)
            .await
    }

    pub(crate) async fn generated_videos_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<GeneratedVideoRecord>, AppError> {
        self.video
            .generated_videos_for_chapter(project_id, chapter_id)
            .await
    }

    pub(crate) async fn generated_video_by_id(
        &self,
        video_id: i64,
    ) -> Result<Option<GeneratedVideoRecord>, AppError> {
        self.video.generated_video_by_id(video_id).await
    }

    pub(crate) async fn generated_videos_by_ids(
        &self,
        video_ids: &[i64],
    ) -> Result<Vec<GeneratedVideoRecord>, AppError> {
        self.video.generated_videos_by_ids(video_ids).await
    }

    pub(crate) async fn resolve_video_config_for_provider_job(
        &self,
        project_id: Option<i64>,
        provider_kind: &str,
        provider_job_id: &str,
    ) -> Result<a4n_provider_contract::VideoProviderConfig, AppError> {
        let provider_job_id = provider_job_id.trim();
        if provider_job_id.is_empty() {
            return Err(AppError::InvalidArgument(
                "provider_job_id 不能为空".to_string(),
            ));
        }
        let provider_kind = provider_kind.trim();
        if provider_kind.is_empty() {
            return Err(AppError::InvalidArgument(
                "provider_kind 不能为空".to_string(),
            ));
        }
        let videos = self
            .video
            .generated_videos_by_provider_job_id(project_id, provider_job_id)
            .await?;
        let video = match videos.as_slice() {
            [] => {
                return Err(AppError::NotFound(format!(
                    "找不到 provider job 本地 identity: {provider_job_id}"
                )));
            }
            [video] => video,
            _ => {
                return Err(AppError::InvalidState {
                    current: "duplicate_provider_job_identity".to_string(),
                    operation: "恢复视频 provider job".to_string(),
                });
            }
        };
        let config_id = crate::video::provider_poll_config_id(video)
            .map_err(AppError::ConfigError)
            .and_then(|id| {
                id.ok_or_else(|| AppError::InvalidState {
                    current: "missing_video_config_identity".to_string(),
                    operation: "恢复视频 provider job".to_string(),
                })
            })?;
        let config = self.video_config_by_id(config_id).await?;
        let capabilities =
            a4n_video::video_provider_capabilities(&config.backend).ok_or_else(|| {
                AppError::ConfigError(format!("未知视频生成后端: {}", config.backend))
            })?;
        if !capabilities.asynchronous {
            return Err(AppError::InvalidState {
                current: "synchronous_video_backend".to_string(),
                operation: "恢复异步视频 provider job".to_string(),
            });
        }
        // Existing persisted checkpoints use the domain marker `video`; new
        // checkpoints may carry the concrete backend and must match exactly.
        if provider_kind != "video" && provider_kind != config.backend {
            return Err(AppError::InvalidState {
                current: format!("provider_kind={provider_kind}"),
                operation: format!("恢复 backend={}", config.backend),
            });
        }
        crate::video::video_provider_config_from_record(&config).map_err(|error| {
            AppError::InvalidState {
                current: error,
                operation: "恢复视频 Provider 价格配置".to_string(),
            }
        })
    }

    pub(crate) async fn update_generated_video_record(
        &self,
        video_id: i64,
        input: UpdateGeneratedVideoRecordInput,
    ) -> Result<GeneratedVideoRecord, AppError> {
        self.video
            .update_generated_video_record(video_id, input)
            .await
    }

    pub(crate) async fn delete_generated_image_record_with_cleanup(
        &self,
        image_id: i64,
    ) -> Result<a4n_db::video::ImageReferenceCleanupIds, AppError> {
        self.video
            .delete_generated_image_record_with_cleanup(image_id)
            .await
    }

    pub(crate) async fn delete_generated_video_record_with_cleanup(
        &self,
        video_id: i64,
    ) -> Result<a4n_db::video::VideoReferenceCleanupIds, AppError> {
        self.video
            .delete_generated_video_record_with_cleanup(video_id)
            .await
    }

    pub(crate) async fn delete_missing_generated_video_references(
        &self,
        video_id: i64,
    ) -> Result<a4n_db::video::VideoReferenceCleanupIds, AppError> {
        self.video
            .delete_missing_generated_video_references_record(video_id)
            .await
    }

    pub(crate) async fn missing_video_reference_project_ids(
        &self,
        video_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        if let Some(video) = self.video.generated_video_by_id(video_id).await? {
            return Ok(vec![video.project_id]);
        }
        self.video
            .missing_video_reference_project_ids(video_id)
            .await
    }

    /// 内部写路径：无状态字面量门禁；对外请用 `*_validated`。
    pub(crate) async fn update_generated_video_waiting_status_and_prompt(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
    ) -> Result<GeneratedVideoRecord, AppError> {
        self.video
            .update_generated_video_waiting_status_and_prompt_record(video_id, prompt_id, status)
            .await
    }

    /// 通用 error 状态写路径（导入失败等无 generation_run 合同场景）。
    /// provider 失败请用 `mark_generated_video_failed_validated`；中断请用 `mark_generated_video_interrupted_validated`。
    pub(crate) async fn mark_generated_video_error_status_and_prompt(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
        message: &str,
    ) -> Result<GeneratedVideoRecord, AppError> {
        self.video
            .mark_generated_video_error_status_and_prompt_record(
                video_id, prompt_id, status, message,
            )
            .await
    }

    /// 内部写路径：无 job id 门禁；对外请用 `*_validated`。
    pub(crate) async fn mark_generated_video_job_and_prompt(
        &self,
        video_id: i64,
        provider_job_id: &str,
        metadata_json: Option<Value>,
        prompt_id: Option<i64>,
    ) -> Result<GeneratedVideoRecord, AppError> {
        self.video
            .mark_generated_video_job_and_prompt_record(
                video_id,
                provider_job_id,
                metadata_json,
                prompt_id,
            )
            .await
    }

    /// provider 异步 job 提交后落库：校验 video_id 与非空 job id。
    /// Host 仍负责后续 wait/poll 与 workflow 钩子。
    pub(crate) async fn mark_generated_video_job_and_prompt_validated(
        &self,
        video_id: i64,
        provider_job_id: &str,
        metadata_json: Option<Value>,
        prompt_id: Option<i64>,
    ) -> Result<GeneratedVideoRecord, AppError> {
        crate::video::ensure_positive_generated_video_id(video_id)
            .map_err(AppError::InvalidArgument)?;
        let provider_job_id = crate::video::required_provider_job_id(provider_job_id)
            .map_err(AppError::InvalidArgument)?;
        self.mark_generated_video_job_and_prompt(
            video_id,
            provider_job_id,
            metadata_json,
            prompt_id,
        )
        .await
    }

    pub(crate) async fn mark_generated_video_completed_and_link_prompt(
        &self,
        video_id: i64,
        file_path: &str,
        thumbnail_path: Option<&str>,
        properties: &a4n_domain::media::VideoFileProperties,
        metadata_json: Option<Value>,
        prompt_id: Option<i64>,
    ) -> Result<GeneratedVideoRecord, AppError> {
        self.video
            .mark_generated_video_completed_and_link_prompt_record(
                video_id,
                file_path,
                thumbnail_path,
                properties,
                metadata_json,
                prompt_id,
            )
            .await
    }

    /// poll/失败路径：校验后将视频标 failed，并 fail 关联 generation_run。
    /// Host 仍负责 workflow 钩子与文件清理。
    pub(crate) async fn mark_generated_video_failed_validated(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        message: &str,
    ) -> Result<GeneratedVideoRecord, AppError> {
        crate::video::ensure_positive_generated_video_id(video_id)
            .map_err(AppError::InvalidArgument)?;
        let message = crate::video::required_generated_video_failure_message(message)
            .map_err(AppError::InvalidArgument)?;
        let updated = self
            .mark_generated_video_error_status_and_prompt(
                video_id,
                prompt_id,
                a4n_domain::video_workflow::VIDEO_STATUS_FAILED,
                message,
            )
            .await?;
        let updated_run_count = self
            .generation
            .fail_generation_runs_for_target("generated_video", video_id, message.to_string(), None)
            .await?;
        crate::video::ensure_provider_generation_run_updated(
            video_id,
            updated_run_count,
            "标记 provider 视频生成失败",
        )
        .map_err(AppError::InvalidArgument)?;
        Ok(updated)
    }

    /// provider 提交前由用户取消：视频、提示词和关联 generation_run 统一标 cancelled。
    pub(crate) async fn mark_generated_video_cancelled_validated(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        message: &str,
    ) -> Result<GeneratedVideoRecord, AppError> {
        crate::video::ensure_positive_generated_video_id(video_id)
            .map_err(AppError::InvalidArgument)?;
        let message = crate::video::required_generated_video_failure_message(message)
            .map_err(AppError::InvalidArgument)?;
        let updated = self
            .mark_generated_video_error_status_and_prompt(
                video_id,
                prompt_id,
                a4n_domain::video_workflow::VIDEO_STATUS_CANCELLED,
                message,
            )
            .await?;
        let updated_run_count = self
            .generation
            .cancel_generation_runs_for_target("generated_video", video_id, message.to_string())
            .await?;
        crate::video::ensure_provider_generation_run_updated(
            video_id,
            updated_run_count,
            "取消 provider 视频生成",
        )
        .map_err(AppError::InvalidArgument)?;
        Ok(updated)
    }

    /// poll 等待态：仅允许 pending/running，再更新视频与提示词状态。
    /// Host 仍负责 workflow running 钩子。
    pub(crate) async fn update_generated_video_waiting_status_validated(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
    ) -> Result<GeneratedVideoRecord, AppError> {
        crate::video::ensure_positive_generated_video_id(video_id)
            .map_err(AppError::InvalidArgument)?;
        crate::video::validate_generated_video_waiting_status(status)
            .map_err(AppError::InvalidArgument)?;
        self.update_generated_video_waiting_status_and_prompt(video_id, prompt_id, status)
            .await
    }

    /// 中断轮询/取消：校验后标 interrupted（不强制 generation_run）。
    /// Host 仍负责 workflow interrupted 钩子。
    pub(crate) async fn mark_generated_video_interrupted_validated(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        message: &str,
    ) -> Result<GeneratedVideoRecord, AppError> {
        crate::video::ensure_positive_generated_video_id(video_id)
            .map_err(AppError::InvalidArgument)?;
        let message = crate::video::required_generated_video_failure_message(message)
            .map_err(AppError::InvalidArgument)?;
        self.mark_generated_video_error_status_and_prompt(
            video_id,
            prompt_id,
            a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED,
            message,
        )
        .await
    }

    /// 完成 provider 视频后：complete 关联 generation_run，并要求至少一条。
    pub(crate) async fn complete_generated_video_provider_runs_validated(
        &self,
        video: &GeneratedVideoRecord,
    ) -> Result<(), AppError> {
        crate::video::ensure_positive_generated_video_id(video.id)
            .map_err(AppError::InvalidArgument)?;
        let updated_run_count = self
            .generation
            .complete_generation_runs_for_target(
                "generated_video",
                video.id,
                Some(serde_json::json!({
                    "generated_video_id": video.id,
                    "file_path": video.file_path,
                    "thumbnail_path": video.thumbnail_path,
                    "file_size": video.file_size,
                    "status": video.status,
                    "provider_result": video.metadata_json,
                })),
            )
            .await?;
        crate::video::ensure_provider_generation_run_updated(
            video.id,
            updated_run_count,
            "完成 provider 视频生成",
        )
        .map_err(AppError::InvalidArgument)?;
        Ok(())
    }
}
