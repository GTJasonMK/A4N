use a4n_core::{error::AppError, models::ai::image_gen_config};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use super::records::image_config_by_id;

pub async fn default_image_config(
    db: &DatabaseConnection,
) -> Result<image_gen_config::Model, AppError> {
    image_gen_config::Entity::find()
        .filter(image_gen_config::Column::IsDefault.eq(true))
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::ConfigError("请先设置默认图片配置".to_string()))
}

pub async fn resolve_image_config(
    db: &DatabaseConnection,
    config_id: Option<i64>,
) -> Result<image_gen_config::Model, AppError> {
    match config_id {
        Some(id) => image_config_by_id(db, id).await,
        None => default_image_config(db).await,
    }
}
