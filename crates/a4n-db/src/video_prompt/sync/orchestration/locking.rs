use a4n_core::models::chapter::chapter;
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, QuerySelect};

use crate::video::VideoDbError;

pub(super) async fn lock_chapter_video_prompt_sync_scope<C>(
    db: &C,
    chapter_id: i64,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    if db.get_database_backend() == DbBackend::Sqlite {
        return Ok(());
    }
    chapter::Entity::find_by_id(chapter_id)
        .lock_exclusive()
        .one(db)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("章节不存在: {chapter_id}")))?;
    Ok(())
}
