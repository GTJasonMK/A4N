use crate::video::VideoDbError;
use a4n_db_models::models::{ai::generated_video, project::novel_project};
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, FromQueryResult, QuerySelect};

#[derive(Debug, FromQueryResult)]
pub(super) struct GeneratedVideoDeleteContext {
    pub(super) id: i64,
    pub(super) project_id: i64,
    pub(super) file_path: Option<String>,
    pub(super) thumbnail_path: Option<String>,
}

pub(super) async fn generated_video_for_delete<C>(
    db: &C,
    video_id: i64,
) -> Result<GeneratedVideoDeleteContext, VideoDbError>
where
    C: ConnectionTrait,
{
    let video_columns = |query: sea_orm::Select<generated_video::Entity>| {
        query
            .select_only()
            .column(generated_video::Column::Id)
            .column(generated_video::Column::ProjectId)
            .column(generated_video::Column::FilePath)
            .column(generated_video::Column::ThumbnailPath)
            .into_model::<GeneratedVideoDeleteContext>()
    };
    if db.get_database_backend() == DbBackend::Sqlite {
        return video_columns(generated_video::Entity::find_by_id(video_id))
            .one(db)
            .await?
            .ok_or_else(|| VideoDbError::NotFound("视频不存在".to_string()));
    }
    let candidate_project_id = generated_video::Entity::find_by_id(video_id)
        .select_only()
        .column(generated_video::Column::ProjectId)
        .into_tuple::<i64>()
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("视频不存在".to_string()))?;
    novel_project::Entity::find_by_id(candidate_project_id)
        .select_only()
        .column(novel_project::Column::Id)
        .lock_exclusive()
        .into_tuple::<i64>()
        .one(db)
        .await?;
    let locked = video_columns(generated_video::Entity::find_by_id(video_id).lock_exclusive())
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("视频不存在".to_string()))?;
    if locked.project_id != candidate_project_id {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频 {video_id} 的项目归属在删除期间发生变化，请重试"
        )));
    }
    Ok(locked)
}
