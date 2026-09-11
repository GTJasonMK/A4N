use super::{DefaultProviderConcurrencyLimits, QueueDbError};
use a4n_db_models::models::ai::{image_gen_config, llm_config, video_gen_config};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect};

pub async fn load_default_provider_concurrency_limits(
    db: &DatabaseConnection,
) -> Result<DefaultProviderConcurrencyLimits, QueueDbError> {
    let (llm, image, video) = futures::join!(
        load_default_llm_concurrency(db),
        load_default_image_concurrency(db),
        load_default_video_concurrency(db),
    );
    Ok(DefaultProviderConcurrencyLimits {
        llm: llm?,
        image: image?,
        video: video?,
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
        .select_only()
        .column(llm_config::Column::MaxConcurrent)
        .filter(llm_config::Column::IsDefault.eq(true))
        .into_tuple::<i32>()
        .one(db)
        .await?;
    config
        .map(|max_concurrent| positive_concurrency(max_concurrent, "默认 LLM 配置"))
        .unwrap_or(Ok(4))
}

async fn load_default_image_concurrency(db: &DatabaseConnection) -> Result<usize, QueueDbError> {
    let config = image_gen_config::Entity::find()
        .select_only()
        .column(image_gen_config::Column::MaxConcurrent)
        .filter(image_gen_config::Column::IsDefault.eq(true))
        .into_tuple::<i32>()
        .one(db)
        .await?;
    config
        .map(|max_concurrent| positive_concurrency(max_concurrent, "默认图片配置"))
        .unwrap_or(Ok(2))
}

async fn load_default_video_concurrency(db: &DatabaseConnection) -> Result<usize, QueueDbError> {
    let config = video_gen_config::Entity::find()
        .select_only()
        .column(video_gen_config::Column::MaxConcurrent)
        .filter(video_gen_config::Column::IsDefault.eq(true))
        .into_tuple::<i32>()
        .one(db)
        .await?;
    config
        .map(|max_concurrent| positive_concurrency(max_concurrent, "默认视频配置"))
        .unwrap_or(Ok(1))
}
