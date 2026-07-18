use super::super::persistence::{
    insert_prompt_from_sync_draft, record_chapter_video_prompt_generation_run,
    replace_prompt_from_sync_draft,
};
use super::preparation::{PlannedPromptSyncRow, PreparedPromptSync};
use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::ConnectionTrait;

use crate::video::{self, VideoDbError};

pub(super) async fn apply_prepared_prompt_sync<C>(
    db: &C,
    prepared: PreparedPromptSync,
) -> Result<Vec<chapter_video_prompt::Model>, VideoDbError>
where
    C: ConnectionTrait,
{
    let PreparedPromptSync {
        draft_count,
        duplicate_rows,
        planned_rows,
        stale_rows,
    } = prepared;

    if !duplicate_rows.is_empty() {
        let duplicate_ids = duplicate_rows.iter().map(|row| row.id).collect::<Vec<_>>();
        video::delete_chapter_video_prompt_records(db, &duplicate_ids).await?;
    }
    let now = chrono::Utc::now().naive_utc();
    let mut rows = Vec::with_capacity(draft_count);
    let mut trace_bindings = Vec::new();
    for PlannedPromptSyncRow {
        existing,
        draft,
        preserve_existing,
    } in planned_rows
    {
        let generation_trace = if preserve_existing {
            None
        } else {
            draft.generation_trace.clone()
        };
        let row = if let Some(existing) = existing {
            if preserve_existing {
                existing
            } else {
                replace_prompt_from_sync_draft(db, existing, draft, now).await?
            }
        } else {
            insert_prompt_from_sync_draft(db, draft, now).await?
        };
        if let Some(trace) = generation_trace {
            trace_bindings.push((row.clone(), trace));
        }
        rows.push(row);
    }
    let stale_ids = stale_rows.iter().map(|row| row.id).collect::<Vec<_>>();
    if !stale_ids.is_empty() {
        video::delete_chapter_video_prompt_records(db, &stale_ids).await?;
    }
    for (row, trace) in trace_bindings {
        record_chapter_video_prompt_generation_run(db, &row, trace).await?;
    }
    Ok(rows)
}
