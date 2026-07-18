use super::identity::should_keep_unmatched_prompt;
use super::*;

pub(super) async fn preserved_unmatched_prompt_ids_by_policy<C>(
    db: &C,
    existing_by_identity: &HashMap<PromptIdentity, chapter_video_prompt::Model>,
    draft_identities: &BTreeSet<PromptIdentity>,
    draft_scopes: &BTreeSet<PromptScope>,
    policy: &ChapterVideoPromptSyncPolicy,
) -> Result<BTreeSet<i64>, VideoDbError>
where
    C: ConnectionTrait,
{
    if !policy.preserve_frame_prompts_with_generated_images {
        return Ok(BTreeSet::new());
    }
    let mut ids = BTreeSet::new();
    for (identity, row) in existing_by_identity {
        if draft_identities.contains(identity) || should_keep_unmatched_prompt(row, draft_scopes) {
            continue;
        }
        if is_sync_frame_prompt_type(&row.prompt_type)
            && chapter_video_prompt_has_generated_frame_image(db, row).await?
        {
            ids.insert(row.id);
        }
    }
    Ok(ids)
}

pub(super) async fn should_preserve_existing_prompt<C>(
    db: &C,
    existing: &chapter_video_prompt::Model,
    draft: &ChapterVideoPromptSyncDraft,
    policy: &ChapterVideoPromptSyncPolicy,
) -> Result<bool, VideoDbError>
where
    C: ConnectionTrait,
{
    if policy.preserve_generated_video_prompts
        && existing.prompt_type == "video_prompt"
        && draft.fields.prompt_type == "video_prompt"
        && existing.generated_video_id.is_some()
        && !prompt_sync_draft_fields_match(existing, draft)
    {
        return Ok(true);
    }
    if policy.preserve_frame_prompts_with_generated_images
        && existing.prompt_type == draft.fields.prompt_type
        && is_sync_frame_prompt_type(&existing.prompt_type)
        && !prompt_sync_draft_fields_match(existing, draft)
        && chapter_video_prompt_has_generated_frame_image(db, existing).await?
    {
        return Ok(true);
    }
    Ok(false)
}

fn is_sync_frame_prompt_type(prompt_type: &str) -> bool {
    matches!(
        prompt_type,
        "frame_prompt" | "first_frame_prompt" | "chat_video_frame_prompt" | "storyboard_prompt"
    )
}

pub(super) fn prompt_sync_draft_fields_match(
    existing: &chapter_video_prompt::Model,
    draft: &ChapterVideoPromptSyncDraft,
) -> bool {
    let fields = &draft.fields;
    existing.project_id == fields.project_id
        && existing.chapter_id == fields.chapter_id
        && existing.style == fields.style
        && existing.prompt_type == fields.prompt_type
        && existing.workflow_mode == fields.workflow_mode
        && existing.shot_number == fields.shot_number
        && existing.title == fields.title
        && existing.scene_summary == fields.scene_summary
        && existing.visual_prompt == fields.visual_prompt
        && existing.motion_prompt == fields.motion_prompt
        && existing.negative_prompt == fields.negative_prompt
        && existing.duration_secs == fields.duration_secs
        && existing.aspect_ratio == fields.aspect_ratio
        && existing.reference_image_ids_json == fields.reference_image_ids_json
        && existing.metadata_json == fields.metadata_json
}

pub(super) fn prompt_sync_generation_fields_changed(
    existing: &chapter_video_prompt::Model,
    draft: &ChapterVideoPromptSyncDraft,
) -> bool {
    let fields = &draft.fields;
    existing.visual_prompt != fields.visual_prompt
        || existing.motion_prompt != fields.motion_prompt
        || existing.negative_prompt != fields.negative_prompt
        || existing.duration_secs != fields.duration_secs
        || existing.aspect_ratio != fields.aspect_ratio
        || existing.reference_image_ids_json != fields.reference_image_ids_json
        || !prompt_sync_metadata_generation_context_matches(
            existing.metadata_json.as_ref(),
            fields.metadata_json.as_ref(),
        )
}

fn prompt_sync_metadata_generation_context_matches(
    existing: Option<&Value>,
    draft: Option<&Value>,
) -> bool {
    existing.and_then(|metadata| metadata.get("source"))
        == draft.and_then(|metadata| metadata.get("source"))
        && existing
            .and_then(|metadata| metadata.get("contract_version"))
            .and_then(Value::as_str)
            == draft
                .and_then(|metadata| metadata.get("contract_version"))
                .and_then(Value::as_str)
}
