use crate::scene_profile::{SceneProfileDbError, SceneProfileDbResult};
use a4n_core::models::chapter::chapter;
use sea_orm::{ConnectionTrait, EntityTrait};

pub(in crate::scene_profile) async fn ensure_chapter_in<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
) -> SceneProfileDbResult<()>
where
    C: ConnectionTrait,
{
    if project_id <= 0 || chapter_id <= 0 || chapter_number <= 0 {
        return Err(SceneProfileDbError::InvalidArgument(
            "场景同步 project_id/chapter_id/chapter_number 必须大于 0".to_string(),
        ));
    }
    let chapter = chapter::Entity::find_by_id(chapter_id)
        .one(db)
        .await?
        .ok_or_else(|| SceneProfileDbError::NotFound(format!("章节不存在: {chapter_id}")))?;
    if chapter.project_id != project_id || chapter.chapter_number != chapter_number {
        return Err(SceneProfileDbError::InvalidArgument(format!(
            "章节 #{chapter_id} 不属于项目 {project_id} 的第 {chapter_number} 章"
        )));
    }
    Ok(())
}
