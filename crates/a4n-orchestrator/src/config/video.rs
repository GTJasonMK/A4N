use a4n_core::{error::AppError, models::ai::video_gen_config};

use crate::{
    OrchestratorContext,
    config::{VideoConfigCreateRecordInput, VideoConfigUpdateRecordInput},
};

impl OrchestratorContext {
    pub async fn video_config(&self, config_id: i64) -> Result<video_gen_config::Model, AppError> {
        a4n_db::config::video_config_by_id(self.db.connection(), config_id).await
    }

    pub async fn list_video_config_records(
        &self,
    ) -> Result<Vec<video_gen_config::Model>, AppError> {
        a4n_db::config::list_video_config_records(self.db.connection()).await
    }

    pub async fn video_config_record_by_id(
        &self,
        config_id: i64,
    ) -> Result<video_gen_config::Model, AppError> {
        a4n_db::config::video_config_record_by_id(self.db.connection(), config_id).await
    }

    pub async fn create_video_config_record(
        &self,
        input: VideoConfigCreateRecordInput,
    ) -> Result<video_gen_config::Model, AppError> {
        a4n_db::config::create_video_config_record(self.db.connection(), input).await
    }

    pub async fn update_video_config_record(
        &self,
        existing: video_gen_config::Model,
        input: VideoConfigUpdateRecordInput,
    ) -> Result<video_gen_config::Model, AppError> {
        a4n_db::config::update_video_config_record(self.db.connection(), existing, input).await
    }

    pub async fn set_default_video_config_record(
        &self,
        model: video_gen_config::Model,
    ) -> Result<video_gen_config::Model, AppError> {
        a4n_db::config::set_default_video_config_record(self.db.connection(), model).await
    }

    pub async fn delete_video_config_record(&self, config_id: i64) -> Result<(), AppError> {
        a4n_db::config::delete_video_config_record(self.db.connection(), config_id).await
    }

    pub async fn default_video_config(&self) -> Result<video_gen_config::Model, AppError> {
        a4n_db::config::default_video_config(self.db.connection()).await
    }

    pub async fn active_generated_video_ids_for_config(
        &self,
        config_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::config::active_generated_video_ids_for_config(self.db.connection(), config_id).await
    }
}
