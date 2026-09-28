//! Provider ports still required by generation workflows.

use super::*;
use a4n_config_read_core::image_provider_config as image_provider_config_from_model;

#[async_trait::async_trait]
impl VideoProviderConfigPort for OrchestratorContext {
    async fn by_id(&self, config_id: i64) -> Result<VideoProviderConfig, AppError> {
        self.video_config_by_id(config_id)
            .await
            .and_then(video_provider_config_checked)
    }

    async fn default(&self) -> Result<VideoProviderConfig, AppError> {
        self.default_video_config()
            .await
            .and_then(video_provider_config_checked)
    }

    async fn for_provider_job(
        &self,
        project_id: Option<i64>,
        provider_kind: &str,
        provider_job_id: &str,
    ) -> Result<VideoProviderConfig, AppError> {
        self.resolve_video_config_for_provider_job(project_id, provider_kind, provider_job_id)
            .await
    }
}

pub(in crate::context) fn video_provider_config_checked(
    config: video_gen_config::Model,
) -> Result<VideoProviderConfig, AppError> {
    video_provider_config_from_model(&config).map_err(|error| AppError::InvalidState {
        current: error,
        operation: "读取视频 Provider 价格配置".to_string(),
    })
}

#[async_trait::async_trait]
impl a4n_api_contract::image::ImageProviderConfigPort for OrchestratorContext {
    async fn by_id(&self, config_id: i64) -> Result<ImageProviderConfig, AppError> {
        self.image_config_by_id(config_id)
            .await
            .and_then(image_provider_config_from_model)
    }

    async fn default(&self) -> Result<ImageProviderConfig, AppError> {
        self.resolve_image_config(None)
            .await
            .and_then(image_provider_config_from_model)
    }
}
