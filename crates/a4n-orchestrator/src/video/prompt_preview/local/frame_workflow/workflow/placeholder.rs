use a4n_core::error::AppError;

use crate::video::OrchestratorContext;
use crate::video::prompt_preview::VideoPromptRequestPreview;
use crate::video::prompt_preview::preview_steps::{
    placeholder_segment_plan, placeholder_segments, placeholder_step_from_request,
};

use super::super::super::planning::SegmentPlanPreviewStepsInput;
use super::super::super::requests::{
    FramePromptPreviewRequestInput, build_boundary_frame_prompt_preview_user,
    build_first_frame_prompt_preview_user, build_frame_prompt_preview_request,
    build_storyboard_prompt_preview_user,
};
use super::super::super::spec::LocalFramePromptPreviewMode;
use super::contracts::FrameWorkflowContext;
use super::projection::preview_from_frame_steps;

pub(super) async fn build_placeholder_frame_preview(
    context: FrameWorkflowContext<'_, '_>,
) -> Result<VideoPromptRequestPreview, AppError> {
    let mut steps = Vec::new();
    context
        .service
        .push_segment_plan_preview_steps(
            &mut steps,
            SegmentPlanPreviewStepsInput {
                prompt_renderer: context.prompt_renderer,
                project_id: context.project_id,
                chapter_id: context.chapter_id,
                source: context.source,
                input: context.input,
                project_context: context.input.project_context.as_ref(),
                segment_title: context.spec.segment_title,
            },
        )
        .await?;
    let placeholder_segment = placeholder_segment_plan();
    let placeholder_segment_plan = placeholder_segments();
    let frame_user = match context.mode {
        LocalFramePromptPreviewMode::BoundaryFrames => {
            let next = OrchestratorContext::placeholder_boundary_frame_prompt_preview_state(
                context.input,
                &placeholder_segment_plan,
            )?;
            build_boundary_frame_prompt_preview_user(
                context.prompt_renderer,
                context.project_id,
                context.source,
                context.input,
                &placeholder_segment,
                &next.current_frame,
                &next.available_assets,
                &next.reference_ids,
            )
            .await?
        }
        LocalFramePromptPreviewMode::FirstFrameChain => {
            build_first_frame_prompt_preview_user(
                context.prompt_renderer,
                context.project_id,
                context.source,
                context.input,
                &placeholder_segment,
            )
            .await?
        }
        LocalFramePromptPreviewMode::Storyboard => {
            let next = OrchestratorContext::placeholder_storyboard_prompt_preview_state(
                context.input,
                &placeholder_segment_plan,
            )?;
            build_storyboard_prompt_preview_user(
                context.prompt_renderer,
                context.project_id,
                context.source,
                context.input,
                &placeholder_segment,
                &next.current_frame,
                &next.available_assets,
            )
            .await?
        }
    };
    let frame_request = build_frame_prompt_preview_request(FramePromptPreviewRequestInput {
        prompt_renderer: context.prompt_renderer,
        project_id: context.project_id,
        source: context.source,
        input: context.input,
        prompt_name: context.spec.frame_prompt_name,
        user: frame_user,
        segment_json: &placeholder_segment,
        asset_context: &context.input.available_assets,
        segment_placeholder: true,
    })
    .await?;
    steps.push(placeholder_step_from_request(
        context.spec.frame_key,
        context.spec.frame_title,
        context.spec.frame_prompt_name,
        frame_request,
        "结构预览：真实生成时 draft 会替换为上一步视频片段规划模型输出；这里用占位片段展示将发送给模型的请求格式。",
    )?);
    Ok(preview_from_frame_steps(context.mode, steps))
}
