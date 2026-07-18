use a4n_core::models::ai::generated_video;
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};

pub async fn generated_video_by_id<C>(
    db: &C,
    video_id: i64,
) -> Result<Option<generated_video::Model>, DbErr>
where
    C: ConnectionTrait,
{
    generated_video::Entity::find_by_id(video_id).one(db).await
}

pub async fn generated_videos_for_chapter<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<generated_video::Model>, DbErr>
where
    C: ConnectionTrait,
{
    generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .filter(generated_video::Column::ChapterId.eq(chapter_id))
        .order_by_desc(generated_video::Column::CreatedAt)
        .all(db)
        .await
}
