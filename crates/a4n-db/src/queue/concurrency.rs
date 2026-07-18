use super::{DefaultProviderConcurrencyLimits, QueueDbError};
use a4n_core::models::ai::{image_gen_config, llm_config, video_gen_config};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

pub async fn load_default_provider_concurrency_limits(
    db: &DatabaseConnection,
) -> Result<DefaultProviderConcurrencyLimits, QueueDbError> {
    Ok(DefaultProviderConcurrencyLimits {
        llm: load_default_llm_concurrency(db).await?,
        image: load_default_image_concurrency(db).await?,
        video: load_default_video_concurrency(db).await?,
    })
}

pub(super) fn positive_concurrency(value: i32, label: &str) -> Result<usize, QueueDbError> {
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| QueueDbError::InvalidConfig(format!("{label} max_concurrent 必须大于 0")))
}

async fn load_default_llm_concurrency(db: &DatabaseConnection) -> Result<usize, QueueDbError> {
    let config = llm_config::Entity::find()
        .filter(llm_config::Column::IsDefault.eq(true))
        .one(db)
        .await?;
    config
        .map(|config| positive_concurrency(config.max_concurrent, "默认 LLM 配置"))
        .unwrap_or(Ok(4))
}

async fn load_default_image_concurrency(db: &DatabaseConnection) -> Result<usize, QueueDbError> {
    let config = image_gen_config::Entity::find()
        .filter(image_gen_config::Column::IsDefault.eq(true))
        .one(db)
        .await?;
    config
        .map(|config| positive_concurrency(config.max_concurrent, "默认图片配置"))
        .unwrap_or(Ok(2))
}

async fn load_default_video_concurrency(db: &DatabaseConnection) -> Result<usize, QueueDbError> {
    let config = video_gen_config::Entity::find()
        .filter(video_gen_config::Column::IsDefault.eq(true))
        .one(db)
        .await?;
    config
        .map(|config| positive_concurrency(config.max_concurrent, "默认视频配置"))
        .unwrap_or(Ok(1))
}
