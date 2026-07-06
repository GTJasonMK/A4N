use a4n_core::error::AppError;
use a4n_core::models::ai::{image_gen_config, llm_config, video_gen_config};
use sea_orm::*;

pub(super) struct QueueConcurrencyLimits {
    pub(super) llm: usize,
    pub(super) image: usize,
    pub(super) video: usize,
    pub(super) embedding: usize,
    pub(super) background: usize,
}

impl QueueConcurrencyLimits {
    pub(super) async fn load(db: &DatabaseConnection) -> Result<Self, AppError> {
        Ok(Self {
            llm: load_default_llm_concurrency(db).await?,
            image: load_default_image_concurrency(db).await?,
            video: load_default_video_concurrency(db).await?,
            embedding: 2,
            background: 2,
        })
    }
}

fn positive_concurrency(value: i32, label: &str) -> Result<usize, AppError> {
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| AppError::ConfigError(format!("{label} max_concurrent 必须大于 0")))
}

async fn load_default_llm_concurrency(db: &DatabaseConnection) -> Result<usize, AppError> {
    let config = llm_config::Entity::find()
        .filter(llm_config::Column::IsDefault.eq(true))
        .one(db)
        .await
        .map_err(AppError::Database)?;
    config
        .map(|config| positive_concurrency(config.max_concurrent, "默认 LLM 配置"))
        .unwrap_or(Ok(4))
}

async fn load_default_image_concurrency(db: &DatabaseConnection) -> Result<usize, AppError> {
    let config = image_gen_config::Entity::find()
        .filter(image_gen_config::Column::IsDefault.eq(true))
        .one(db)
        .await
        .map_err(AppError::Database)?;
    config
        .map(|config| positive_concurrency(config.max_concurrent, "默认图片配置"))
        .unwrap_or(Ok(2))
}

async fn load_default_video_concurrency(db: &DatabaseConnection) -> Result<usize, AppError> {
    let config = video_gen_config::Entity::find()
        .filter(video_gen_config::Column::IsDefault.eq(true))
        .one(db)
        .await
        .map_err(AppError::Database)?;
    config
        .map(|config| positive_concurrency(config.max_concurrent, "默认视频配置"))
        .unwrap_or(Ok(1))
}
