use super::super::{chapter_video_prompt_record, chapter_video_prompt_records_by_ids};
use super::rules::ensure_chapter_video_prompt_not_running;
use crate::video::VideoDbError;
use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use std::collections::BTreeSet;

pub async fn delete_chapter_video_prompt_record<C>(
    db: &C,
    prompt_id: i64,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let prompt = chapter_video_prompt_record(db, prompt_id).await?;
    ensure_chapter_video_prompt_not_running(&prompt, "删除视频提示词")?;
    crate::generation::delete_generation_run_targets_for(db, "chapter_video_prompt", &[prompt_id])
        .await?;
    chapter_video_prompt::Entity::delete_by_id(prompt_id)
        .exec(db)
        .await?;
    Ok(())
}

pub async fn delete_chapter_video_prompt_records<C>(
    db: &C,
    prompt_ids: &[i64],
) -> Result<u64, VideoDbError>
where
    C: ConnectionTrait,
{
    let prompt_ids = prompt_ids.iter().copied().collect::<BTreeSet<_>>();
    if prompt_ids.is_empty() {
        return Ok(0);
    }
    let prompt_ids_vec = prompt_ids.iter().copied().collect::<Vec<_>>();
    let rows = chapter_video_prompt_records_by_ids(db, &prompt_ids_vec).await?;
    if rows.len() != prompt_ids.len() {
        return Err(VideoDbError::NotFound(
            "部分视频提示词不存在，不能批量删除".to_string(),
        ));
    }
    for row in &rows {
        ensure_chapter_video_prompt_not_running(row, "删除视频提示词")?;
    }
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
    Ok(result.rows_affected)
}
