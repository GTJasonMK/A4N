use a4n_core::{error::AppError, models::ai::generated_video};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub async fn active_generated_video_ids_for_config<C>(
    db: &C,
    config_id: i64,
) -> Result<Vec<i64>, AppError>
where
    C: ConnectionTrait,
{
    let mut active: Vec<_> = generated_video::Entity::find()
        .filter(generated_video::Column::Status.is_in(["pending", "running"]))
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .filter(|video| {
            video
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("config_id"))
                .and_then(|value| value.as_i64())
                == Some(config_id)
        })
        .collect();
    active.sort_by_key(|video| (video.created_at, video.id));
    Ok(active.into_iter().map(|video| video.id).collect())
}
