//! Generation-time configuration reads; user mutations belong to Config Worker.

use super::*;
use a4n_domain::config::{
    EmbeddingConfigReadRecord, ImageConfigReadRecord, LlmConfigReadRecord, VideoConfigReadRecord,
};

impl ConfigApplication {
    pub(in crate::context) async fn start_model_usage_record(
        &self,
        input: ModelUsageRecordStart,
    ) -> Result<i64, AppError> {
        a4n_db::model_usage::start_model_usage_record_id(self.database.executor(), input).await
    }

    pub(in crate::context) async fn finish_model_usage_record(
        &self,
        record_id: i64,
        input: ModelUsageRecordCompletion,
    ) -> Result<(), AppError> {
        a4n_db::model_usage::finish_model_usage_record(self.database.executor(), record_id, input)
            .await
    }

    pub(in crate::context) async fn resolve_llm_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<LlmConfigReadRecord, AppError> {
        a4n_db::config::read_records::resolve_llm_config_read_record(
            self.database.executor(),
            config_id,
        )
        .await
    }

    pub(in crate::context) async fn image_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<ImageConfigReadRecord, AppError> {
        a4n_db::config::read_records::image_config_read_record_by_id(
            self.database.executor(),
            config_id,
        )
        .await
    }

    pub(in crate::context) async fn resolve_image_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<ImageConfigReadRecord, AppError> {
        a4n_db::config::read_records::resolve_image_config_read_record(
            self.database.executor(),
            config_id,
        )
        .await
    }

    pub(in crate::context) async fn video_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<VideoConfigReadRecord, AppError> {
        a4n_db::config::read_records::video_config_read_record_by_id(
            self.database.executor(),
            config_id,
        )
        .await
    }

    pub(in crate::context) async fn default_video_config(
        &self,
    ) -> Result<VideoConfigReadRecord, AppError> {
        a4n_db::config::read_records::default_video_config_read_record(self.database.executor())
            .await
    }

    pub(in crate::context) async fn active_generated_video_ids_for_config(
        &self,
        config_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::config::active_generated_video_ids_for_config(self.database.executor(), config_id)
            .await
    }

    pub(in crate::context) async fn default_embedding_config_optional(
        &self,
    ) -> Result<Option<EmbeddingConfigReadRecord>, AppError> {
        a4n_db::config::read_records::default_embedding_config_read_record_optional(
            self.database.executor(),
        )
        .await
    }

    pub(in crate::context) async fn default_embedding_config(
        &self,
    ) -> Result<EmbeddingConfigReadRecord, AppError> {
        a4n_db::config::read_records::default_embedding_config_read_record(self.database.executor())
            .await
    }
}
