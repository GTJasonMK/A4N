//! AI配置管理服务：LLM/嵌入/图片/视频生成配置的 CRUD 和默认管理
//!
//! 对应文档 13-LLM配置系统、14-嵌入模型配置系统

macro_rules! config_record_read_methods {
    (
        entity = $entity:path,
        model = $model:path,
        list = $list_fn:ident => $query:expr,
        get = $get_fn:ident,
        not_found = $not_found:expr $(,)?
    ) => {
        pub async fn $list_fn(&self) -> Result<Vec<$model>, AppError> {
            $query
                .all(self.db.as_ref())
                .await
                .map_err(AppError::Database)
        }

        pub async fn $get_fn(&self, config_id: i64) -> Result<$model, AppError> {
            <$entity as sea_orm::EntityTrait>::find_by_id(config_id)
                .one(self.db.as_ref())
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| AppError::NotFound($not_found(config_id)))
        }
    };
}

macro_rules! config_record_crud_methods {
    (
        entity = $entity:path,
        model = $model:path,
        list = $list_fn:ident => $query:expr,
        get = $get_fn:ident,
        delete = $delete_fn:ident,
        default = $default_fn:ident -> $default_return:ty => $map_default:expr,
        not_found = $not_found:expr,
        validate = $validate:expr,
        before_delete = $before_delete:expr,
        default_column = $default_column:path $(,)?
    ) => {
        config_record_read_methods!(
            entity = $entity,
            model = $model,
            list = $list_fn => $query,
            get = $get_fn,
            not_found = $not_found,
        );

        pub async fn $default_fn(&self, config_id: i64) -> Result<$default_return, AppError> {
            let cfg = self.$get_fn(config_id).await?;
            $validate(&cfg)?;
            let updated =
                super::set_single_default::<$entity, _>(self.db.as_ref(), cfg, $default_column)
                    .await?;
            Ok(($map_default)(updated))
        }

        pub async fn $delete_fn(&self, config_id: i64) -> Result<(), AppError> {
            let cfg = self.$get_fn(config_id).await?;
            $before_delete(&cfg)?;
            <$entity as sea_orm::EntityTrait>::delete_by_id(config_id)
                .exec(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
            Ok(())
        }
    };
}

fn ensure_default_config_delete_allowed(
    kind: &str,
    is_default: bool,
) -> Result<(), a4n_core::error::AppError> {
    if is_default {
        return Err(a4n_core::error::AppError::InvalidArgument(format!(
            "不能删除默认{kind}，请先设置其它默认配置"
        )));
    }
    Ok(())
}

mod embedding;
mod image;
mod llm;
mod validation;
mod video;

#[cfg(test)]
mod tests;

pub use llm::{CreateLlmConfigInput, UpdateLlmConfigInput};
pub(crate) use validation::normalize_grok_reference_mode_options_json;
pub use validation::{
    validate_embedding_config_params, validate_image_config_model, validate_image_config_params,
};
pub use video::active_generated_video_ids_for_config;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct CreateEmbeddingConfigParams {
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,
    pub local_model_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateEmbeddingConfigParams {
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key_encrypted: Option<Option<String>>,
    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,
    pub local_model_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateImageConfigParams {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub provider_options_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct UpdateImageConfigParams {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    pub api_key_encrypted: Option<Option<String>>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub provider_options_json: Option<serde_json::Value>,
}

#[derive(Clone)]
pub struct ConfigService {
    db: Arc<DatabaseConnection>,
}

impl ConfigService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    async fn ensure_config_name_available<E>(
        &self,
        kind: &str,
        name: &str,
        name_column: E::Column,
        id_column: E::Column,
        except_id: Option<i64>,
    ) -> Result<(), a4n_core::error::AppError>
    where
        E: EntityTrait,
        E::Model: sea_orm::FromQueryResult + Send + Sync + 'static,
        E::Column: Copy,
    {
        let mut query = E::find().filter(name_column.eq(name));
        if let Some(id) = except_id {
            query = query.filter(id_column.ne(id));
        }
        validation::ensure_name_available(kind, name, query.count(self.db.as_ref()).await?)
    }
}

async fn set_single_default<E, C>(
    db: &C,
    model: E::Model,
    is_default_column: E::Column,
) -> Result<E::Model, a4n_core::error::AppError>
where
    E: sea_orm::EntityTrait,
    E::Model: sea_orm::IntoActiveModel<E::ActiveModel>,
    E::ActiveModel: sea_orm::ActiveModelTrait<Entity = E> + Send,
    E::Column: Copy,
    C: sea_orm::ConnectionTrait,
{
    set_single_default_with::<E, C, _>(db, model, is_default_column, |_| {}).await
}

async fn set_single_default_with<E, C, F>(
    db: &C,
    model: E::Model,
    is_default_column: E::Column,
    update_active: F,
) -> Result<E::Model, a4n_core::error::AppError>
where
    E: sea_orm::EntityTrait,
    E::Model: sea_orm::IntoActiveModel<E::ActiveModel>,
    E::ActiveModel: sea_orm::ActiveModelTrait<Entity = E> + Send,
    E::Column: Copy,
    C: sea_orm::ConnectionTrait,
    F: FnOnce(&mut E::ActiveModel),
{
    E::update_many()
        .col_expr(is_default_column, sea_orm::sea_query::Expr::value(false))
        .filter(is_default_column.eq(true))
        .exec(db)
        .await
        .map_err(a4n_core::error::AppError::Database)?;

    let mut active = model.into_active_model();
    active.set(is_default_column, sea_orm::Value::Bool(Some(true)));
    update_active(&mut active);
    active
        .update(db)
        .await
        .map_err(a4n_core::error::AppError::Database)
}
