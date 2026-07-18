use crate::video::VideoDbError;
use a4n_core::models::{ai::generated_video, project::novel_project};
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, QuerySelect};

pub(super) async fn generated_video_for_delete<C>(
    db: &C,
    video_id: i64,
) -> Result<generated_video::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let candidate = generated_video::Entity::find_by_id(video_id)
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("视频不存在".to_string()))?;
    if db.get_database_backend() == DbBackend::Sqlite {
        return Ok(candidate);
    }

    novel_project::Entity::find_by_id(candidate.project_id)
        .lock_exclusive()
        .one(db)
        .await?;
    let locked = generated_video::Entity::find_by_id(video_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("视频不存在".to_string()))?;
    if locked.project_id != candidate.project_id {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频 {video_id} 的项目归属在删除期间发生变化，请重试"
        )));
    }
    Ok(locked)
}
