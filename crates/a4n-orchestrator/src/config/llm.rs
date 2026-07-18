use a4n_core::{error::AppError, models::ai::llm_config};

use crate::{
    OrchestratorContext,
    config::{LlmConfigCreateRecordInput, LlmConfigUpdateRecordInput},
};

impl OrchestratorContext {
    pub async fn list_llm_config_records(&self) -> Result<Vec<llm_config::Model>, AppError> {
        a4n_db::config::list_llm_config_records(self.db.connection()).await
    }

    pub async fn llm_config_record_by_id(
        &self,
        config_id: i64,
    ) -> Result<llm_config::Model, AppError> {
        a4n_db::config::llm_config_record_by_id(self.db.connection(), config_id).await
    }

    pub async fn create_llm_config_record(
        &self,
        input: LlmConfigCreateRecordInput,
    ) -> Result<llm_config::Model, AppError> {
        a4n_db::config::create_llm_config_record(self.db.connection(), input).await
    }

    pub async fn update_llm_config_record(
        &self,
        existing: llm_config::Model,
        input: LlmConfigUpdateRecordInput,
    ) -> Result<llm_config::Model, AppError> {
        a4n_db::config::update_llm_config_record(self.db.connection(), existing, input).await
    }

    pub async fn set_default_llm_config_record(
        &self,
        model: llm_config::Model,
    ) -> Result<llm_config::Model, AppError> {
        a4n_db::config::set_default_llm_config_record(self.db.connection(), model).await
    }

    pub async fn delete_llm_config_record(&self, config_id: i64) -> Result<(), AppError> {
        a4n_db::config::delete_llm_config_record(self.db.connection(), config_id).await
    }

    pub async fn resolve_llm_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<llm_config::Model, AppError> {
        a4n_db::config::resolve_llm_config(self.db.connection(), config_id).await
    }
}
