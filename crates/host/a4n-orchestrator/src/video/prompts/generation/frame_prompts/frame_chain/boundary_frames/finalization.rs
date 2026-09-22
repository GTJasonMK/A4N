use a4n_error::AppError;

use crate::video::prompts::generation::frame_prompts::{
    FramePromptDraftsInput, NewChapterVideoPrompt, frame_prompt_drafts,
    report_video_prompt_progress, validate_frame_prompts, validate_partial_boundary_frame_prompts,
};

use super::contracts::BoundaryFrameFinalizationInput;

pub(in crate::video::prompts::generation::frame_prompts::frame_chain) fn finalize_boundary_frame_batch(
    input: BoundaryFrameFinalizationInput<'_, '_>,
) -> Result<Vec<NewChapterVideoPrompt>, AppError> {
    validate_partial_boundary_frame_prompts(
        input.frame_prompts,
        input.segment_plan,
        input.allowed_reference_ids,
        input.boundary_frame_image_id_by_number,
    )?;
    if input.frame_prompts.len() == input.expected_frame_count {
        validate_frame_prompts(
            input.frame_prompts,
            input.segment_plan,
            input.allowed_reference_ids,
        )?;
    }
    let final_message = if input.frame_prompts.len() == input.expected_frame_count {
        format!(
            "边界帧提示词已全部生成，共 {}/{} 条",
            input.frame_prompts.len(),
            input.expected_frame_count
        )
    } else {
        format!(
            "本批已生成 {} 条边界帧提示词，当前共 {}/{} 条",
            input.generated_this_run,
            input.frame_prompts.len(),
            input.expected_frame_count
        )
    };
    report_video_prompt_progress(input.progress, 90, 100, &final_message)?;
    Ok(frame_prompt_drafts(FramePromptDraftsInput {
        common: input.frame_draft_common,
        workflow_mode: input.spec.workflow_mode,
        prompt_type: input.spec.frame_prompt_type,
        source_prompt_type: input.frame_source_prompt_type,
        frame_prompts: input.frame_prompts,
    }))
}
