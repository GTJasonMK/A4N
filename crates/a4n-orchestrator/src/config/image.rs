use a4n_core::{error::AppError, models::ai::image_gen_config};

use crate::{
    OrchestratorContext,
    config::{ImageConfigCreateRecordInput, ImageConfigUpdateRecordInput},
};

impl OrchestratorContext {
    pub async fn list_image_config_records(
        &self,
    ) -> Result<Vec<image_gen_config::Model>, AppError> {
        a4n_db::config::list_image_config_records(self.db.connection()).await
    }

    pub async fn image_config_record_by_id(
        &self,
        config_id: i64,
    ) -> Result<image_gen_config::Model, AppError> {
        a4n_db::config::image_config_record_by_id(self.db.connection(), config_id).await
    }

    pub async fn create_image_config_record(
        &self,
        input: ImageConfigCreateRecordInput,
    ) -> Result<image_gen_config::Model, AppError> {
        a4n_db::config::create_image_config_record(self.db.connection(), input).await
    }

    pub async fn update_image_config_record(
        &self,
        existing: image_gen_config::Model,
        input: ImageConfigUpdateRecordInput,
    ) -> Result<image_gen_config::Model, AppError> {
        a4n_db::config::update_image_config_record(self.db.connection(), existing, input).await
    }

    pub async fn set_default_image_config_record(
        &self,
        model: image_gen_config::Model,
    ) -> Result<image_gen_config::Model, AppError> {
        a4n_db::config::set_default_image_config_record(self.db.connection(), model).await
    }

    pub async fn delete_image_config_record(&self, config_id: i64) -> Result<(), AppError> {
        a4n_db::config::delete_image_config_record(self.db.connection(), config_id).await
    }

    pub async fn resolve_image_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<image_gen_config::Model, AppError> {
        a4n_db::config::resolve_image_config(self.db.connection(), config_id).await
    }
}
