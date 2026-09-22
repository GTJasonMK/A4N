use std::collections::BTreeSet;

use a4n_error::AppError;

use crate::video::prompts::generation::frame_prompts::{
    prompt_row_draft, report_video_prompt_progress, validate_frame_prompts,
};

use super::super::boundary_frames::prepare_boundary_frame_reuse;
use super::contracts::{
    BoundaryWorkflowPreparationInput, PendingBoundaryWorkflow, PreparedBoundaryWorkflow,
};

pub(super) async fn prepare_boundary_workflow(
    input: BoundaryWorkflowPreparationInput<'_, '_>,
) -> Result<PreparedBoundaryWorkflow, AppError> {
    let segment_plan = input.reuse.segment_plan;
    let prepared = prepare_boundary_frame_reuse(input.reuse).await?;
    let kept_prompt_ids = prepared
        .prefix_rows
        .iter()
        .map(|row| row.id)
        .collect::<BTreeSet<_>>();
    if prepared.frame_prompts.len() == input.expected_frame_count {
        validate_frame_prompts(
            &prepared.frame_prompts,
            segment_plan,
            &prepared.allowed_reference_ids,
        )?;
        report_video_prompt_progress(
            input.progress,
            90,
            100,
            &format!(
                "已复用 {} 条{}，正在写入记录",
                prepared.prefix_rows.len(),
                input.reused_frame_label
            ),
        )?;
        let drafts = prepared
            .prefix_rows
            .iter()
            .map(prompt_row_draft)
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(PreparedBoundaryWorkflow::Complete(drafts));
    }

    Ok(PreparedBoundaryWorkflow::Pending(PendingBoundaryWorkflow {
        boundary_frame_image_id_by_number: prepared.boundary_frame_image_id_by_number,
        allowed_reference_ids: prepared.allowed_reference_ids,
        frame_prompts: prepared.frame_prompts,
        kept_prompt_ids,
    }))
}
