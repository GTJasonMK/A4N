use super::super::{
    identity::{
        prompt_identity_for_sync_draft, prompt_identity_for_sync_row, prompt_scope_for_sync_draft,
        should_keep_unmatched_prompt, should_prefer_existing_prompt,
    },
    preservation::{
        preserved_unmatched_prompt_ids_by_policy, prompt_sync_draft_fields_match,
        segment_plan_extension_preserves_existing, should_preserve_existing_prompt,
    },
    validate_prompt_sync_draft,
    validation::{
        validate_prompt_output_language_metadata, validate_prompt_workflow_mode,
    },
};
use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::ConnectionTrait;
use std::collections::{BTreeSet, HashMap};

use crate::video::{self, VideoDbError};
use crate::video_prompt::{
    ChapterVideoPromptSyncDraft, ChapterVideoPromptSyncPolicy, PromptIdentity,
    ensure_chapter_video_prompt_stage_mutable, ensure_chapter_video_prompt_stage_mutable_ignoring,
    lock_chapter_video_prompt_scope,
};

pub(super) struct PlannedPromptSyncRow {
    pub(super) existing: Option<chapter_video_prompt::Model>,
    pub(super) draft: ChapterVideoPromptSyncDraft,
    pub(super) preserve_existing: bool,
}

pub(super) struct PreparedPromptSync {
    pub(super) draft_count: usize,
    pub(super) duplicate_rows: Vec<chapter_video_prompt::Model>,
    pub(super) planned_rows: Vec<PlannedPromptSyncRow>,
    pub(super) stale_rows: Vec<chapter_video_prompt::Model>,
}

pub(super) async fn prepare_prompt_sync<C>(
    db: &C,
    chapter_id: i64,
    drafts: Vec<ChapterVideoPromptSyncDraft>,
    policy: &ChapterVideoPromptSyncPolicy,
) -> Result<PreparedPromptSync, VideoDbError>
where
    C: ConnectionTrait,
{
    let project_id = drafts[0].fields.project_id;
    lock_chapter_video_prompt_scope(db, chapter_id).await?;
    video::ensure_video_chapter_belongs_to_project_record(db, project_id, chapter_id).await?;
    for draft in &drafts {
        validate_prompt_sync_draft(project_id, chapter_id, draft)?;
        video::validated_video_reference_image_records(
            db,
            project_id,
            chapter_id,
            draft.fields.reference_image_ids_json.as_ref(),
        )
        .await?;
    }

    let draft_count = drafts.len();
    let prompt_types = drafts
        .iter()
        .map(|draft| draft.fields.prompt_type.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let draft_workflows = drafts
        .iter()
        .map(|draft| draft.fields.workflow_mode.clone())
        .collect::<BTreeSet<_>>();
    let draft_workflow_modes = draft_workflows.iter().cloned().collect::<Vec<_>>();
    let draft_identities = drafts
        .iter()
        .map(prompt_identity_for_sync_draft)
        .collect::<BTreeSet<_>>();
    let draft_scopes = drafts
        .iter()
        .map(prompt_scope_for_sync_draft)
        .collect::<BTreeSet<_>>();
    let existing = video::chapter_video_prompt_sync_candidates(
        db,
        project_id,
        chapter_id,
        &prompt_types,
        &draft_workflow_modes,
    )
    .await
    .map_err(VideoDbError::Database)?;
    let mut existing_by_identity = HashMap::<PromptIdentity, chapter_video_prompt::Model>::new();
    let mut duplicate_rows = Vec::new();
    for row in existing {
        validate_prompt_workflow_mode(&row.workflow_mode, row.id)?;
        validate_prompt_output_language_metadata(
            &row.prompt_type,
            row.metadata_json.as_ref(),
            row.id,
        )?;
        if !draft_workflows.contains(&row.workflow_mode) {
            continue;
        }
        let identity = prompt_identity_for_sync_row(&row);
        if let Some(current) = existing_by_identity.get(&identity) {
            if should_prefer_existing_prompt(&row, current) {
                duplicate_rows.push(current.clone());
                existing_by_identity.insert(identity, row);
            } else {
                duplicate_rows.push(row);
            }
            continue;
        }
        existing_by_identity.insert(identity, row);
    }
    for row in &duplicate_rows {
        video::ensure_chapter_video_prompt_not_running(row, "删除视频提示词")?;
        ensure_chapter_video_prompt_stage_mutable(db, row, "删除").await?;
    }

    let preserved_unmatched_prompt_ids = preserved_unmatched_prompt_ids_by_policy(
        db,
        &existing_by_identity,
        &draft_identities,
        &draft_scopes,
        policy,
    )
    .await?;
    let replace_prompt_ids = existing_by_identity
        .iter()
        .filter(|(identity, _)| draft_identities.contains(identity))
        .map(|(_, row)| row.id)
        .collect::<BTreeSet<_>>();
    let delete_prompt_ids = existing_by_identity
        .iter()
        .filter(|(identity, row)| {
            !draft_identities.contains(identity)
                && !should_keep_unmatched_prompt(row, &draft_scopes)
                && !preserved_unmatched_prompt_ids.contains(&row.id)
        })
        .map(|(_, row)| row.id)
        .chain(duplicate_rows.iter().map(|row| row.id))
        .collect::<BTreeSet<_>>();
    let ignored_prompt_ids = replace_prompt_ids
        .union(&delete_prompt_ids)
        .copied()
        .collect::<BTreeSet<_>>();

    let mut planned_rows = Vec::with_capacity(draft_count);
    for draft in drafts {
        let identity = prompt_identity_for_sync_draft(&draft);
        if let Some(existing) = existing_by_identity.remove(&identity) {
            let preserve_existing =
                should_preserve_existing_prompt(db, &existing, &draft, policy).await?;
            if !preserve_existing
                && !prompt_sync_draft_fields_match(&existing, &draft)
                && !segment_plan_extension_preserves_existing(&existing, &draft)
            {
                ensure_chapter_video_prompt_stage_mutable_ignoring(
                    db,
                    &existing,
                    "重新生成",
                    &ignored_prompt_ids,
                )
                .await?;
            }
            planned_rows.push(PlannedPromptSyncRow {
                existing: Some(existing),
                draft,
                preserve_existing,
            });
        } else {
            planned_rows.push(PlannedPromptSyncRow {
                existing: None,
                draft,
                preserve_existing: false,
            });
        }
    }

    let stale_rows = existing_by_identity
        .into_values()
        .filter(|row| {
            !should_keep_unmatched_prompt(row, &draft_scopes)
                && !preserved_unmatched_prompt_ids.contains(&row.id)
        })
        .collect::<Vec<_>>();
    for row in &stale_rows {
        video::ensure_chapter_video_prompt_not_running(row, "删除视频提示词")?;
        ensure_chapter_video_prompt_stage_mutable(db, row, "删除").await?;
    }

    Ok(PreparedPromptSync {
        draft_count,
        duplicate_rows,
        planned_rows,
        stale_rows,
    })
}
