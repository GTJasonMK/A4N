use a4n_core::{error::AppError, models::ai::embedding_config};

use crate::{
    OrchestratorContext,
    config::{EmbeddingConfigCreateRecordInput, EmbeddingConfigUpdateRecordInput},
};

impl OrchestratorContext {
    pub async fn default_embedding_config_optional(
        &self,
    ) -> Result<Option<embedding_config::Model>, AppError> {
        a4n_db::config::default_embedding_config_optional(self.db.connection()).await
    }

    pub async fn default_embedding_config(&self) -> Result<embedding_config::Model, AppError> {
        a4n_db::config::default_embedding_config(self.db.connection()).await
    }

    pub async fn list_embedding_configs(&self) -> Result<Vec<embedding_config::Model>, AppError> {
        a4n_db::config::list_embedding_configs(self.db.connection()).await
    }

    pub async fn embedding_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<embedding_config::Model, AppError> {
        a4n_db::config::embedding_config_by_id(self.db.connection(), config_id).await
    }

    pub async fn create_embedding_config_record(
        &self,
        input: EmbeddingConfigCreateRecordInput,
    ) -> Result<embedding_config::Model, AppError> {
        a4n_db::config::create_embedding_config_record(self.db.connection(), input).await
    }

    pub async fn update_embedding_config_record(
        &self,
        existing: embedding_config::Model,
        input: EmbeddingConfigUpdateRecordInput,
    ) -> Result<embedding_config::Model, AppError> {
        a4n_db::config::update_embedding_config_record(self.db.connection(), existing, input).await
    }

    pub async fn set_default_embedding_config_record(
        &self,
        model: embedding_config::Model,
    ) -> Result<embedding_config::Model, AppError> {
        a4n_db::config::set_default_embedding_config_record(self.db.connection(), model).await
    }

    pub async fn delete_embedding_config_record(&self, config_id: i64) -> Result<(), AppError> {
        a4n_db::config::delete_embedding_config_record(self.db.connection(), config_id).await
    }
}
