use a4n_core::error::AppError;
use a4n_core::models::ai::{image_gen_config, llm_config, video_gen_config};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

pub async fn resolve_llm_config(
    db: &DatabaseConnection,
    config_id: Option<i64>,
) -> Result<llm_config::Model, AppError> {
    match config_id {
        Some(id) => find_llm_config(db, id).await,
        None => find_default_llm_config(db).await,
    }
}

pub async fn resolve_image_config(
    db: &DatabaseConnection,
    config_id: Option<i64>,
) -> Result<image_gen_config::Model, AppError> {
    match config_id {
        Some(id) => find_image_config(db, id).await,
        None => find_default_image_config(db).await,
    }
}

pub async fn resolve_video_config(
    db: &DatabaseConnection,
    config_id: Option<i64>,
) -> Result<video_gen_config::Model, AppError> {
    match config_id {
        Some(id) => find_video_config(db, id).await,
        None => find_default_video_config(db).await,
    }
}

async fn find_llm_config(db: &DatabaseConnection, id: i64) -> Result<llm_config::Model, AppError> {
    llm_config::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("LLM配置不存在: {id}")))
}

async fn find_image_config(
    db: &DatabaseConnection,
    id: i64,
) -> Result<image_gen_config::Model, AppError> {
    image_gen_config::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("图片配置 #{id} 不存在")))
}

async fn find_video_config(
    db: &DatabaseConnection,
    id: i64,
) -> Result<video_gen_config::Model, AppError> {
    video_gen_config::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("视频配置 #{id} 不存在")))
}

async fn find_default_llm_config(db: &DatabaseConnection) -> Result<llm_config::Model, AppError> {
    llm_config::Entity::find()
        .filter(llm_config::Column::IsDefault.eq(true))
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::ConfigError("请先设置默认 LLM 配置".to_string()))
}

async fn find_default_image_config(
    db: &DatabaseConnection,
) -> Result<image_gen_config::Model, AppError> {
    image_gen_config::Entity::find()
        .filter(image_gen_config::Column::IsDefault.eq(true))
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::ConfigError("请先设置默认图片配置".to_string()))
}

async fn find_default_video_config(
    db: &DatabaseConnection,
) -> Result<video_gen_config::Model, AppError> {
    video_gen_config::Entity::find()
        .filter(video_gen_config::Column::IsDefault.eq(true))
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::ConfigError("请先设置默认视频配置".to_string()))
}
