//! provider 执行 port。

use super::*;
use async_trait::async_trait;

#[async_trait]
impl VideoProviderExecutionApplicationPort for crate::OrchestratorContext {
    async fn poll_target(
        &self,
        user_id: i64,
        video_id: i64,
    ) -> Result<VideoProviderPollTarget, AppError> {
        let video = self.required_video_record(video_id).await?;
        self.verify_project_permission(
            video.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        if !crate::video::has_active_provider_job(video.provider_job_id.as_deref(), &video.status) {
            return Ok(VideoProviderPollTarget::Inactive);
        }
        Ok(VideoProviderPollTarget::Active {
            config_id: crate::video::provider_poll_config_id(&video)
                .map_err(AppError::InvalidArgument)?,
        })
    }

    async fn poll_provider_job(
        &self,
        user_id: i64,
        video_id: i64,
        config: Option<&VideoProviderConfig>,
        host_port: &dyn VideoProviderHostPort,
    ) -> Result<GeneratedVideoSummary, AppError> {
        let video = self.required_video_record(video_id).await?;
        self.verify_project_permission(
            video.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        if !crate::video::has_active_provider_job(video.provider_job_id.as_deref(), &video.status) {
            return Ok(generated_video_summary(&video));
        }
        let config = config.ok_or_else(|| AppError::InvalidState {
            current: "video_provider_poll_config_missing".to_string(),
            operation: "恢复视频 provider 轮询".to_string(),
        })?;
        let prompt_id = a4n_domain::video::generated_video_prompt_id(video.metadata_json.as_ref());
        let updated = self
            .poll_video_provider_once_validated(
                video,
                config,
                prompt_id,
                CancellationToken::new(),
                host_port,
            )
            .await?;
        Ok(generated_video_summary(&updated))
    }
}
