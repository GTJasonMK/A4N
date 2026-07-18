use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
};

use crate::video::{VideoDbError, validate_positive_video_i64};

pub async fn chapter_video_prompt_by_id(
    db: &DatabaseConnection,
    prompt_id: i64,
) -> Result<Option<chapter_video_prompt::Model>, VideoDbError> {
    validate_positive_video_i64(prompt_id, "prompt_id")?;
    chapter_video_prompt::Entity::find_by_id(prompt_id)
        .one(db)
        .await
        .map_err(VideoDbError::Database)
}

pub async fn chapter_video_prompts_for_chapter_by_id(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<chapter_video_prompt::Model>, VideoDbError> {
    validate_positive_video_i64(project_id, "project_id")?;
    validate_positive_video_i64(chapter_id, "chapter_id")?;
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}

pub async fn chapter_video_prompts_for_chapter<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<chapter_video_prompt::Model>, DbErr>
where
    C: ConnectionTrait,
{
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .order_by_asc(chapter_video_prompt::Column::WorkflowMode)
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
}

pub async fn chapter_video_prompt_record<C>(
    db: &C,
    prompt_id: i64,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    chapter_video_prompt_record_with_message(db, prompt_id, "视频提示词不存在").await
}

pub async fn frame_chapter_video_prompt_record<C>(
    db: &C,
    prompt_id: i64,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    chapter_video_prompt_record_with_message(db, prompt_id, "视频帧提示词不存在").await
}

async fn chapter_video_prompt_record_with_message<C>(
    db: &C,
    prompt_id: i64,
    not_found_message: &str,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    chapter_video_prompt::Entity::find_by_id(prompt_id)
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound(not_found_message.to_string()))
}

pub async fn chapter_video_prompt_records_by_ids<C>(
    db: &C,
    prompt_ids: &[i64],
) -> Result<Vec<chapter_video_prompt::Model>, VideoDbError>
where
    C: ConnectionTrait,
{
    if prompt_ids.is_empty() {
        return Ok(Vec::new());
    }
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::Id.is_in(prompt_ids.iter().copied()))
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}
