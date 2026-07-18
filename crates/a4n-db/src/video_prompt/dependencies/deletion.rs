use super::mutability::{
    ensure_chapter_video_prompt_stage_mutable, ensure_chapter_video_prompt_stage_mutable_ignoring,
};
use crate::video::{self, VideoDbError};
use sea_orm::{ConnectionTrait, TransactionTrait};
use std::collections::BTreeSet;

pub async fn delete_chapter_video_prompt_with_policy<C>(
    db: &C,
    prompt_id: i64,
    allow_downstream: bool,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let prompt = video::chapter_video_prompt_record(&txn, prompt_id).await?;
    if prompt.status == "running" {
        return Err(VideoDbError::InvalidState {
            current: prompt.status,
            operation: "删除视频提示词".to_string(),
        });
    }
    if !allow_downstream {
        ensure_chapter_video_prompt_stage_mutable(&txn, &prompt, "删除").await?;
    }
    video::delete_chapter_video_prompt_record(&txn, prompt_id).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(())
}

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
    let prompt_ids_vec = prompt_ids_set.iter().copied().collect::<Vec<_>>();
    let rows = video::chapter_video_prompt_records_by_ids(&txn, &prompt_ids_vec).await?;
    if rows.len() != prompt_ids_set.len() {
        return Err(VideoDbError::NotFound(
            "部分视频提示词不存在，不能批量删除".into(),
        ));
    }
    for row in &rows {
        if row.status == "running" {
            return Err(VideoDbError::InvalidState {
                current: row.status.clone(),
                operation: "删除视频提示词".to_string(),
            });
        }
        if !allow_downstream {
            ensure_chapter_video_prompt_stage_mutable_ignoring(&txn, row, "删除", &prompt_ids_set)
                .await?;
        }
    }
    let deleted = video::delete_chapter_video_prompt_records(&txn, &prompt_ids_vec).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(deleted)
}
