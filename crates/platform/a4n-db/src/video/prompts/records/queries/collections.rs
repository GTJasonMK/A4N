use a4n_db_models::models::chapter::chapter_video_prompt;
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect};

pub async fn chapter_video_prompt_records_for_types<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    prompt_types: &[&str],
) -> Result<Vec<chapter_video_prompt::Model>, DbErr>
where
    C: ConnectionTrait,
{
    if prompt_types.is_empty() {
        return Ok(Vec::new());
    }
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::PromptType.is_in(prompt_types.iter().copied()))
        .all(db)
        .await
}

pub async fn chapter_video_prompt_ids_for_types<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    prompt_types: &[&str],
) -> Result<Vec<i64>, DbErr>
where
    C: ConnectionTrait,
{
    if prompt_types.is_empty() {
        return Ok(Vec::new());
    }
    chapter_video_prompt::Entity::find()
        .select_only()
        .column(chapter_video_prompt::Column::Id)
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::PromptType.is_in(prompt_types.iter().copied()))
        .into_tuple::<i64>()
        .all(db)
        .await
}
