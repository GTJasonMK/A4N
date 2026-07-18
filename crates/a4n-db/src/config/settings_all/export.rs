use a4n_core::error::AppError;
use a4n_core::models::{
    ai::{embedding_config, image_gen_config, llm_config, system_settings, video_gen_config},
    user::theme_config,
};
use sea_orm::{DatabaseConnection, EntityTrait};

pub async fn settings_all_export_llm_configs(
    db: &DatabaseConnection,
) -> Result<Vec<llm_config::Model>, AppError> {
    llm_config::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)
}

pub async fn settings_all_export_embedding_configs(
    db: &DatabaseConnection,
) -> Result<Vec<embedding_config::Model>, AppError> {
    embedding_config::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)
}

pub async fn settings_all_export_image_configs(
    db: &DatabaseConnection,
) -> Result<Vec<image_gen_config::Model>, AppError> {
    image_gen_config::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)
}

pub async fn settings_all_export_video_configs(
    db: &DatabaseConnection,
) -> Result<Vec<video_gen_config::Model>, AppError> {
    video_gen_config::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)
}

pub async fn settings_all_export_themes(
    db: &DatabaseConnection,
) -> Result<Vec<theme_config::Model>, AppError> {
    theme_config::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)
}

pub async fn settings_all_export_system_settings(
    db: &DatabaseConnection,
) -> Result<Vec<system_settings::Model>, AppError> {
    system_settings::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)
}
