use a4n_core::{error::AppError, models::ai::video_gen_config};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use super::records::video_config_by_id;

pub async fn default_video_config(
    db: &DatabaseConnection,
) -> Result<video_gen_config::Model, AppError> {
    video_gen_config::Entity::find()
        .filter(video_gen_config::Column::IsDefault.eq(true))
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::ConfigError("请先设置默认视频配置".to_string()))
}

pub async fn resolve_video_config(
    db: &DatabaseConnection,
    config_id: Option<i64>,
) -> Result<video_gen_config::Model, AppError> {
    match config_id {
        Some(id) => video_config_by_id(db, id).await,
        None => default_video_config(db).await,
    }
}
