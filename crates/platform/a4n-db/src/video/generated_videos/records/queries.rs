use a4n_db_models::models::ai::generated_video;
use a4n_domain::media::GeneratedVideoRecord;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};

use crate::video::media_mapping::generated_video_record_from_model;

pub async fn generated_video_by_id<C>(
    db: &C,
    video_id: i64,
) -> Result<Option<GeneratedVideoRecord>, DbErr>
where
    C: ConnectionTrait,
{
    generated_video::Entity::find_by_id(video_id)
        .one(db)
        .await
        .map(|model| model.map(generated_video_record_from_model))
}

pub async fn generated_videos_by_ids<C: ConnectionTrait>(
    db: &C,
    video_ids: &[i64],
) -> Result<Vec<GeneratedVideoRecord>, DbErr> {
    if video_ids.is_empty() {
        return Ok(Vec::new());
    }
    let models = generated_video::Entity::find()
        .filter(generated_video::Column::Id.is_in(video_ids.iter().copied()))
        .all(db)
        .await?;
    Ok(models
        .into_iter()
        .map(generated_video_record_from_model)
        .collect())
}

pub async fn generated_videos_for_chapter<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<GeneratedVideoRecord>, DbErr>
where
    C: ConnectionTrait,
{
    let models = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .filter(generated_video::Column::ChapterId.eq(chapter_id))
        .order_by_desc(generated_video::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(models
        .into_iter()
        .map(generated_video_record_from_model)
        .collect())
}

pub async fn generated_videos_by_provider_job_id<C>(
    db: &C,
    project_id: Option<i64>,
    provider_job_id: &str,
) -> Result<Vec<GeneratedVideoRecord>, DbErr>
where
    C: ConnectionTrait,
{
    let provider_job_id = provider_job_id.trim();
    if provider_job_id.is_empty() {
        return Err(DbErr::Custom("provider_job_id 不能为空".to_string()));
    }
    if project_id.is_some_and(|value| value <= 0) {
        return Err(DbErr::Custom(
            "provider job project_id 必须为正整数".to_string(),
        ));
    }
    let mut query = generated_video::Entity::find()
        .filter(generated_video::Column::ProviderJobId.eq(provider_job_id))
        .order_by_asc(generated_video::Column::Id)
        .limit(2);
    if let Some(project_id) = project_id {
        query = query.filter(generated_video::Column::ProjectId.eq(project_id));
    }
    Ok(query
        .all(db)
        .await?
        .into_iter()
        .map(generated_video_record_from_model)
        .collect())
}
