use a4n_core::error::AppError;

use crate::video::prompt_generation::ReusableSegmentPlan;
use crate::video::prompt_preview::VideoPromptRequestPreview;
use crate::video::prompt_preview::preview_steps::reused_step;

use super::super::super::inputs::allowed_reference_ids_for_preview;
use super::super::super::spec::LocalVideoPromptPreviewMode;
use super::boundary::append_boundary_frame_preview_step;
use super::contracts::FrameWorkflowContext;
use super::first_frame::append_first_frame_preview_step;
use super::projection::preview_from_frame_steps;
use super::storyboard::append_storyboard_preview_step;

pub(super) async fn build_reusable_frame_preview(
    context: FrameWorkflowContext<'_, '_>,
    reusable_segment: &ReusableSegmentPlan,
) -> Result<VideoPromptRequestPreview, AppError> {
    let mut steps = vec![reused_step(
        "segment_plan",
        context.spec.segment_title,
        "chapter_video_segment_plan",
        "已存在可复用片段规划，本次生成会跳过该 LLM 请求",
    )];
    let segment_json = &reusable_segment.segment_json;
    let segment_plan = &reusable_segment.segment_plan;
    let allowed_reference_ids = allowed_reference_ids_for_preview(
        &context.input.available_assets,
        &context.input.reference_image_ids,
    )?;
    match context.spec.mode {
        LocalVideoPromptPreviewMode::BoundaryFrames
        | LocalVideoPromptPreviewMode::ChatVideoBoundaryFrames => {
            append_boundary_frame_preview_step(
                context,
                segment_json,
                segment_plan,
                &allowed_reference_ids,
                &reusable_segment.execution_identity,
                &mut steps,
            )
            .await?;
        }
        LocalVideoPromptPreviewMode::FirstFrameChain => {
            append_first_frame_preview_step(
                context,
                segment_json,
                segment_plan,
                &allowed_reference_ids,
                &reusable_segment.execution_identity,
                &mut steps,
            )
            .await?;
        }
        LocalVideoPromptPreviewMode::Storyboard => {
            append_storyboard_preview_step(
                context,
                segment_json,
                segment_plan,
                &allowed_reference_ids,
                &reusable_segment.execution_identity,
                &mut steps,
            )
            .await?;
        }
    }
    Ok(preview_from_frame_steps(context.spec.mode, steps))
}
