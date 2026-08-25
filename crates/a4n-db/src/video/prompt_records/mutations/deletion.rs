use crate::video::VideoDbError;
use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use std::collections::HashSet;

pub(crate) async fn delete_chapter_video_prompt_records<C>(
    db: &C,
    prompts: &[chapter_video_prompt::Model],
) -> Result<u64, VideoDbError>
where
    C: ConnectionTrait,
{
    let prompt_ids = prompts
        .iter()
        .map(|prompt| prompt.id)
        .collect::<HashSet<_>>();
    if prompt_ids.is_empty() {
        return Ok(0);
    }
    let prompt_ids_vec = prompt_ids.iter().copied().collect::<Vec<_>>();
    crate::generation::delete_generation_run_targets_for(
        db,
        "chapter_video_prompt",
        &prompt_ids_vec,
    )
    .await?;
    let result = chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::Id.is_in(prompt_ids.iter().copied()))
        .exec(db)
        .await?;
    if result.rows_affected != prompt_ids.len() as u64 {
        return Err(VideoDbError::NotFound(if prompt_ids.len() == 1 {
            "视频提示词不存在".to_string()
        } else {
            "部分视频提示词不存在，不能批量删除".to_string()
        }));
    }
    Ok(result.rows_affected)
}
