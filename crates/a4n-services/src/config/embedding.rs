use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

use a4n_core::error::AppError;
use a4n_core::models::ai::embedding_config;

use super::validation::{validate_embedding_config_model, validate_embedding_config_params};
use super::{ConfigService, CreateEmbeddingConfigParams, UpdateEmbeddingConfigParams};

impl ConfigService {
    config_record_read_methods!(
        entity = embedding_config::Entity,
        model = embedding_config::Model,
        list = list_embedding => <embedding_config::Entity as sea_orm::EntityTrait>::find(),
        get = get_embedding,
        not_found = |config_id| format!("嵌入配置不存在: {config_id}"),
    );

    pub async fn set_default_embedding(&self, config_id: i64) -> Result<(), AppError> {
        let cfg = self.get_embedding(config_id).await?;
        validate_embedding_config_model(&cfg)?;
        super::set_single_default::<embedding_config::Entity, _>(
            self.db.as_ref(),
            cfg,
            embedding_config::Column::IsDefault,
        )
        .await?;
        Ok(())
    }

    pub async fn delete_embedding(&self, config_id: i64) -> Result<(), AppError> {
        let cfg = self.get_embedding(config_id).await?;
        ensure_embedding_delete_allowed(&cfg)?;
        embedding_config::Entity::delete_by_id(config_id)
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn create_embedding(
        &self,
        params: CreateEmbeddingConfigParams,
    ) -> Result<embedding_config::Model, AppError> {
        let params = validate_embedding_config_params(params)?;
        self.ensure_config_name_available::<embedding_config::Entity>(
            "嵌入配置",
            &params.name,
            embedding_config::Column::Name,
            embedding_config::Column::Id,
            None,
        )
        .await?;
        let now = Utc::now().naive_utc();
        embedding_config::ActiveModel {
            name: Set(params.name),
            backend: Set(params.backend),
            model: Set(params.model),
            dimension: Set(params.dimension),
            batch_size: Set(params.batch_size),
            base_url: Set(params.base_url),
            api_key_encrypted: Set(params.api_key_encrypted),
            local_model_path: Set(params.local_model_path),
            is_default: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)
    }

    pub async fn update_embedding(
        &self,
        config_id: i64,
        params: UpdateEmbeddingConfigParams,
    ) -> Result<embedding_config::Model, AppError> {
        let existing = self.get_embedding(config_id).await?;
        let requested_api_key = params
            .api_key_encrypted
            .unwrap_or_else(|| existing.api_key_encrypted.clone());
        let params = validate_embedding_config_params(CreateEmbeddingConfigParams {
            name: params.name,
            backend: params.backend,
            base_url: params.base_url,
            api_key_encrypted: requested_api_key,
            model: params.model,
            dimension: params.dimension,
            batch_size: params.batch_size,
            local_model_path: params.local_model_path,
        })?;
        let api_key_encrypted = if params.backend == "api" {
            params.api_key_encrypted
        } else {
            None
        };
        self.ensure_config_name_available::<embedding_config::Entity>(
            "嵌入配置",
            &params.name,
            embedding_config::Column::Name,
            embedding_config::Column::Id,
            Some(config_id),
        )
        .await?;

        let mut active: embedding_config::ActiveModel = existing.into();
        active.name = Set(params.name);
        active.backend = Set(params.backend);
        active.model = Set(params.model);
        active.dimension = Set(params.dimension);
        active.batch_size = Set(params.batch_size);
        active.base_url = Set(params.base_url);
        active.api_key_encrypted = Set(api_key_encrypted);
        active.local_model_path = Set(params.local_model_path);
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }
}

fn ensure_embedding_delete_allowed(config: &embedding_config::Model) -> Result<(), AppError> {
    if config.is_default {
        return Err(AppError::InvalidArgument(
            "不能删除默认 Embedding 配置，请先设置其它默认配置".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn embedding_model(is_default: bool) -> embedding_config::Model {
        embedding_config::Model {
            id: 1,
            name: "local".to_string(),
            backend: "local_hash".to_string(),
            base_url: None,
            api_key_encrypted: None,
            model: "local".to_string(),
            dimension: 8,
            batch_size: 100,
            local_model_path: None,
            is_default,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }

    #[test]
    fn embedding_default_config_cannot_be_deleted() {
        let err = ensure_embedding_delete_allowed(&embedding_model(true))
            .expect_err("default embedding config deletion must fail");

        assert!(matches!(
            err,
            AppError::InvalidArgument(message)
                if message == "不能删除默认 Embedding 配置，请先设置其它默认配置"
        ));
    }

    #[test]
    fn embedding_non_default_config_can_be_deleted() {
        ensure_embedding_delete_allowed(&embedding_model(false))
            .expect("non-default embedding config deletion should be allowed");
    }
}
