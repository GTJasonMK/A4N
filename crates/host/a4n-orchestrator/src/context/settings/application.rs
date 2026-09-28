//! Generation-time configuration reads; user mutations belong to Config Worker.

use super::*;

impl ConfigApplication {
    pub(in crate::context) async fn start_model_usage_record(
        &self,
        input: ModelUsageRecordStart,
    ) -> Result<a4n_db_models::models::generation::model_usage_record::Model, AppError> {
        a4n_db::model_usage::start_model_usage_record(self.database.connection(), input).await
    }

    pub(in crate::context) async fn finish_model_usage_record(
        &self,
        record_id: i64,
        input: ModelUsageRecordCompletion,
    ) -> Result<(), AppError> {
        a4n_db::model_usage::finish_model_usage_record(self.database.connection(), record_id, input)
            .await
    }

    pub(in crate::context) async fn resolve_llm_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<llm_config::Model, AppError> {
        a4n_db::config::resolve_llm_config(self.database.connection(), config_id).await
    }

    pub(in crate::context) async fn image_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<image_gen_config::Model, AppError> {
        a4n_db::config::image_config_by_id(self.database.connection(), config_id).await
    }

    pub(in crate::context) async fn resolve_image_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<image_gen_config::Model, AppError> {
        a4n_db::config::resolve_image_config(self.database.connection(), config_id).await
    }

    pub(in crate::context) async fn video_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<video_gen_config::Model, AppError> {
        a4n_db::config::video_config_by_id(self.database.connection(), config_id).await
    }

    pub(in crate::context) async fn default_video_config(
        &self,
    ) -> Result<video_gen_config::Model, AppError> {
        a4n_db::config::default_video_config(self.database.connection()).await
    }

    pub(in crate::context) async fn active_generated_video_ids_for_config(
        &self,
        config_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::config::active_generated_video_ids_for_config(self.database.connection(), config_id)
            .await
    }

    pub(in crate::context) async fn default_embedding_config_optional(
        &self,
    ) -> Result<Option<embedding_config::Model>, AppError> {
        a4n_db::config::default_embedding_config_optional(self.database.connection()).await
    }

    pub(in crate::context) async fn default_embedding_config(
        &self,
    ) -> Result<embedding_config::Model, AppError> {
        a4n_db::config::default_embedding_config(self.database.connection()).await
    }
}
