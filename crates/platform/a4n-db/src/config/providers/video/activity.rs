use a4n_db_models::models::ai::generated_video;
use a4n_error::AppError;
use chrono::NaiveDateTime;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter, QuerySelect,
};
use serde_json::Value;

#[derive(Debug, FromQueryResult)]
struct ActiveGeneratedVideoRow {
    id: i64,
    metadata_json: Option<Value>,
    created_at: NaiveDateTime,
}

pub async fn active_generated_video_ids_for_config<C>(
    db: &C,
    config_id: i64,
) -> Result<Vec<i64>, AppError>
where
    C: ConnectionTrait,
{
    let mut active: Vec<_> = generated_video::Entity::find()
        .select_only()
        .column(generated_video::Column::Id)
        .column(generated_video::Column::MetadataJson)
        .column(generated_video::Column::CreatedAt)
        .filter(generated_video::Column::Status.is_in(["pending", "running"]))
        .into_model::<ActiveGeneratedVideoRow>()
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
