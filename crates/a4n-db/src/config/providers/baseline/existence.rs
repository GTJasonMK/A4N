use a4n_core::{
    error::AppError,
    models::ai::{embedding_config, image_gen_config, llm_config, video_gen_config},
};
use sea_orm::{DatabaseConnection, EntityTrait};

pub(super) async fn embedding_configs_exist(db: &DatabaseConnection) -> Result<bool, AppError> {
    Ok(embedding_config::Entity::find()
        .one(db)
        .await
        .map_err(AppError::Database)?
        .is_some())
}

pub async fn llm_configs_exist(db: &DatabaseConnection) -> Result<bool, AppError> {
    Ok(llm_config::Entity::find()
        .one(db)
        .await
        .map_err(AppError::Database)?
        .is_some())
}

pub async fn image_configs_exist(db: &DatabaseConnection) -> Result<bool, AppError> {
    Ok(image_gen_config::Entity::find()
        .one(db)
        .await
        .map_err(AppError::Database)?
        .is_some())
}

pub async fn video_configs_exist(db: &DatabaseConnection) -> Result<bool, AppError> {
    Ok(video_gen_config::Entity::find()
        .one(db)
        .await
        .map_err(AppError::Database)?
        .is_some())
}
