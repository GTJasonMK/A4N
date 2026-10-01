use super::{application::apply_prepared_prompt_sync, preparation::prepare_prompt_sync};
use sea_orm::{ConnectionTrait, TransactionTrait};

use crate::video::VideoDbError;
use crate::video::prompts::record_mapping::chapter_video_prompt_record_from_model;
use a4n_domain::video_prompt::ChapterVideoPromptRecord;
use a4n_domain::video_prompt::ChapterVideoPromptSyncDraft;
use a4n_domain::video_prompt::ChapterVideoPromptSyncPolicy;

pub async fn sync_chapter_video_prompts_with_generation_runs<C>(
    db: &C,
    chapter_id: i64,
    drafts: Vec<ChapterVideoPromptSyncDraft>,
    policy: ChapterVideoPromptSyncPolicy,
) -> Result<Vec<ChapterVideoPromptRecord>, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    if drafts.is_empty() {
        return Ok(Vec::new());
    }
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let prepared = prepare_prompt_sync(&txn, chapter_id, drafts, &policy).await?;
    let rows = apply_prepared_prompt_sync(&txn, prepared).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(rows
        .into_iter()
        .map(chapter_video_prompt_record_from_model)
        .collect())
}
