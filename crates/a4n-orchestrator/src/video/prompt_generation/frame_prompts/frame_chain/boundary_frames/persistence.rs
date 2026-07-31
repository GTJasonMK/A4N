use super::*;

pub(in crate::video::prompt_generation::frame_prompts::frame_chain) async fn persist_boundary_frame_checkpoint(
    input: BoundaryFramePersistenceInput<'_>,
) -> Result<BTreeSet<i64>, AppError> {
    let BoundaryFramePersistenceInput {
        service,
        spec,
        chapter_id,
        frame_number,
        workflow_run_id,
        frame_draft_common,
        frame_source_prompt_type,
        segment_draft,
        frame_trace,
        frame_prompt,
        frame_prompts,
        allowed_reference_ids,
    } = input;
    allowed_reference_ids.extend(frame_prompt.reference_image_ids.iter().copied());
    frame_prompts.push(frame_prompt);
    frame_prompts.sort_by_key(|frame| frame.frame_number);

    let frame_drafts = frame_prompt_drafts(FramePromptDraftsInput {
        common: frame_draft_common,
        workflow_mode: spec.workflow_mode,
        prompt_type: spec.frame_prompt_type,
        source_prompt_type: frame_source_prompt_type,
        frame_prompts,
    });
    let frame_drafts = attach_generation_trace_to_draft(
        frame_drafts,
        spec.workflow_mode,
        spec.frame_prompt_type,
        frame_number,
        video_prompt_generation_trace(frame_trace, workflow_run_id),
    );
    let mut checkpoint_drafts = Vec::new();
    if let Some(draft) = segment_draft {
        checkpoint_drafts.push(draft.clone());
    }
    checkpoint_drafts.extend(frame_drafts);
    let checkpoint_rows = service
        .sync_chapter_prompts_preserving_generated_frame_prompts(chapter_id, checkpoint_drafts)
        .await?;

    Ok(checkpoint_rows
        .iter()
        .filter(|row| row.prompt_type == spec.frame_prompt_type)
        .map(|row| row.id)
        .collect())
}
