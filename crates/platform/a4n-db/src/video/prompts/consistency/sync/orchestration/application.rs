use super::super::identity::{prompt_identity_for_sync_draft, prompt_identity_for_sync_row};
use super::super::persistence::{
    prompt_active_model, record_chapter_video_prompt_generation_run, replace_prompt_from_sync_draft,
};
use super::preparation::{PlannedPromptSyncRow, PreparedPromptSync};
use a4n_db_models::models::chapter::chapter_video_prompt;
use sea_orm::{ConnectionTrait, EntityTrait};
use std::collections::{HashMap, HashSet, VecDeque};

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
        video::delete_chapter_video_prompt_records(db, &duplicate_rows).await?;
    }
    let now = chrono::Utc::now().naive_utc();
    let mut rows = (0..draft_count).map(|_| None).collect::<Vec<_>>();
    let mut trace_bindings = (0..draft_count).map(|_| None).collect::<Vec<_>>();
    let mut pending_inserts = Vec::new();
    let mut pending_insert_identities = Vec::new();
    for (
        slot,
        PlannedPromptSyncRow {
            existing,
            draft,
            preserve_existing,
        },
    ) in planned_rows.into_iter().enumerate()
    {
        trace_bindings[slot] = if preserve_existing {
            None
        } else {
            draft.generation_trace.clone()
        };
        if let Some(existing) = existing {
            let row = if preserve_existing {
                existing
            } else {
                replace_prompt_from_sync_draft(db, existing, draft, now).await?
            };
            rows[slot] = Some(row);
        } else {
            pending_insert_identities.push((slot, prompt_identity_for_sync_draft(&draft)));
            pending_inserts.push(prompt_active_model(&draft, now));
        }
    }

    if !pending_inserts.is_empty() {
        let expected = pending_inserts.len();
        let inserted = chapter_video_prompt::Entity::insert_many(pending_inserts)
            .exec_with_returning_many(db)
            .await
            .map_err(VideoDbError::Database)?;
        if inserted.len() != expected {
            return Err(VideoDbError::InvalidState {
                current: format!(
                    "视频提示词批量插入返回行数不一致: inserted={}, expected={}",
                    inserted.len(),
                    expected
                ),
                operation: "同步视频提示词".to_string(),
            });
        }
        let mut inserted_by_identity = HashMap::<_, VecDeque<_>>::new();
        for row in inserted {
            inserted_by_identity
                .entry(prompt_identity_for_sync_row(&row))
                .or_default()
                .push_back(row);
        }
        for (slot, identity) in pending_insert_identities {
            rows[slot] = Some(
                inserted_by_identity
                    .get_mut(&identity)
                    .and_then(VecDeque::pop_front)
                    .ok_or_else(|| VideoDbError::InvalidState {
                        current: "视频提示词批量插入后无法按身份回读".to_string(),
                        operation: "同步视频提示词".to_string(),
                    })?,
            );
        }
    }

    if !stale_rows.is_empty() {
        video::delete_chapter_video_prompt_records(db, &stale_rows).await?;
    }

    let mut output_rows = Vec::with_capacity(draft_count);
    let mut completed_trace_bindings = Vec::new();
    for (slot, row) in rows.into_iter().enumerate() {
        let row = row.ok_or_else(|| VideoDbError::InvalidState {
            current: "视频提示词同步缺少返回记录".to_string(),
            operation: "同步视频提示词".to_string(),
        })?;
        if let Some(trace) = trace_bindings[slot].take() {
            completed_trace_bindings.push((row.clone(), trace));
        }
        output_rows.push(row);
    }
    let existing_trace_keys = if completed_trace_bindings.is_empty() {
        HashSet::new()
    } else {
        let target_ids = completed_trace_bindings
            .iter()
            .map(|(row, _)| row.id)
            .collect::<Vec<_>>();
        let step_keys = completed_trace_bindings
            .iter()
            .map(|(_, trace)| trace.prompt_name.clone())
            .collect::<Vec<_>>();
        crate::generation::generation_run_existing_target_outputs(
            db,
            "chapter_video_prompt",
            &target_ids,
            &step_keys,
        )
        .await?
    };
    let mut recorded_trace_keys = existing_trace_keys;
    for (row, trace) in completed_trace_bindings {
        let trace_key = (
            row.id,
            trace.prompt_name.trim().to_string(),
            trace.response_text.clone(),
        );
        if !recorded_trace_keys.insert(trace_key) {
            continue;
        }
        record_chapter_video_prompt_generation_run(db, &row, trace).await?;
    }
    Ok(output_rows)
}
