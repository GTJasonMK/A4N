//! Generation-time configuration reads; user mutations belong to Config Worker.

use super::*;

impl OrchestratorContext {
    pub(crate) async fn image_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<ImageConfigReadRecord, AppError> {
        self.settings.image_config_by_id(config_id).await
    }

    pub(crate) async fn resolve_image_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<ImageConfigReadRecord, AppError> {
        self.settings.resolve_image_config(config_id).await
    }

    pub(crate) async fn video_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<VideoConfigReadRecord, AppError> {
        self.settings.video_config_by_id(config_id).await
    }

    pub(crate) async fn default_video_config(&self) -> Result<VideoConfigReadRecord, AppError> {
        self.settings.default_video_config().await
    }

    pub(crate) async fn active_generated_video_ids_for_config(
        &self,
        config_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        self.settings
            .active_generated_video_ids_for_config(config_id)
            .await
    }
}
