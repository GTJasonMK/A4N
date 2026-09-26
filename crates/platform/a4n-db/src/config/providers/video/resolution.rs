use a4n_db_models::models::ai::video_gen_config;
use a4n_error::AppError;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

pub async fn default_video_config(
    db: &DatabaseConnection,
) -> Result<video_gen_config::Model, AppError> {
    video_gen_config::Entity::find()
        .filter(video_gen_config::Column::IsDefault.eq(true))
        .one(db)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::ConfigError("请先设置默认视频配置".to_string()))
}
