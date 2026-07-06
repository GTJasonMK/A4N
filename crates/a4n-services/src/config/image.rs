use chrono::Utc;
use sea_orm::{ActiveModelTrait, QueryOrder, Set};

use a4n_core::error::AppError;
use a4n_core::models::ai::image_gen_config;

use super::validation::{validate_image_config_model, validate_image_config_params};
use super::{ConfigService, CreateImageConfigParams, UpdateImageConfigParams};

impl ConfigService {
    config_record_crud_methods!(
        entity = image_gen_config::Entity,
        model = image_gen_config::Model,
        list = list_image => <image_gen_config::Entity as sea_orm::EntityTrait>::find()
            .order_by_asc(image_gen_config::Column::Name),
        get = get_image,
        delete = delete_image,
        default = set_default_image -> image_gen_config::Model => |updated| updated,
        not_found = |_| "图片配置不存在".into(),
        validate = |cfg| validate_image_config_model(cfg).map(drop),
        before_delete = |cfg: &image_gen_config::Model| super::ensure_default_config_delete_allowed(
            "图片配置",
            cfg.is_default,
        ),
        default_column = image_gen_config::Column::IsDefault,
    );

    pub async fn create_image(
        &self,
        params: CreateImageConfigParams,
    ) -> Result<image_gen_config::Model, AppError> {
        let params = validate_image_config_params(params)?;
        self.ensure_config_name_available::<image_gen_config::Entity>(
            "图片配置",
            &params.name,
            image_gen_config::Column::Name,
            image_gen_config::Column::Id,
            None,
        )
        .await?;
        let now = Utc::now().naive_utc();
        image_gen_config::ActiveModel {
            name: Set(params.name),
            backend: Set(params.backend),
            base_url: Set(Some(params.base_url)),
            api_key_encrypted: Set(params.api_key_encrypted),
            model: Set(params.model),
            max_concurrent: Set(params.max_concurrent),
            default_width: Set(params.default_width),
            default_height: Set(params.default_height),
            provider_options_json: Set(params.provider_options_json),
            is_default: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)
    }

    pub async fn update_image(
        &self,
        config_id: i64,
        params: UpdateImageConfigParams,
    ) -> Result<image_gen_config::Model, AppError> {
        let existing = self.get_image(config_id).await?;
        let api_key_encrypted = params
            .api_key_encrypted
            .unwrap_or_else(|| existing.api_key_encrypted.clone());
        let params = validate_image_config_params(CreateImageConfigParams {
            name: params.name,
            backend: params.backend,
            base_url: params.base_url,
            api_key_encrypted,
            model: params.model,
            max_concurrent: params.max_concurrent,
            default_width: params.default_width,
            default_height: params.default_height,
            provider_options_json: params.provider_options_json,
        })?;
        self.ensure_config_name_available::<image_gen_config::Entity>(
            "图片配置",
            &params.name,
            image_gen_config::Column::Name,
            image_gen_config::Column::Id,
            Some(config_id),
        )
        .await?;

        let mut active: image_gen_config::ActiveModel = existing.into();
        active.name = Set(params.name);
        active.backend = Set(params.backend);
        active.base_url = Set(Some(params.base_url));
        active.api_key_encrypted = Set(params.api_key_encrypted);
        active.model = Set(params.model);
        active.max_concurrent = Set(params.max_concurrent);
        active.default_width = Set(params.default_width);
        active.default_height = Set(params.default_height);
        active.provider_options_json = Set(params.provider_options_json);
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }
}
