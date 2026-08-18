use super::mutability::ensure_chapter_video_prompt_stage_mutable_ignoring;
use crate::video::{self, VideoDbError};
use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, TransactionTrait};
use std::collections::BTreeSet;

pub async fn delete_chapter_video_prompts_with_policy<C>(
    db: &C,
    prompt_ids: &[i64],
    allow_downstream: bool,
) -> Result<u64, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let prompt_ids_set = prompt_ids.iter().copied().collect::<BTreeSet<_>>();
    if prompt_ids_set.is_empty() {
        return Ok(0);
    }
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let rows = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::Id.is_in(prompt_ids_set.iter().copied()))
        .all(&txn)
        .await
        .map_err(VideoDbError::Database)?;
    if rows.len() != prompt_ids_set.len() {
        return Err(VideoDbError::NotFound(if prompt_ids_set.len() == 1 {
            "视频提示词不存在".to_string()
        } else {
            "部分视频提示词不存在，不能批量删除".to_string()
        }));
    }
    for row in &rows {
        video::ensure_chapter_video_prompt_not_running(row, "删除视频提示词")?;
        if !allow_downstream {
            ensure_chapter_video_prompt_stage_mutable_ignoring(&txn, row, "删除", &prompt_ids_set)
                .await?;
        }
    }
    let deleted = video::delete_chapter_video_prompt_records(&txn, &rows).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(deleted)
}
