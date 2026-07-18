use std::collections::HashSet;

use a4n_core::error::AppError;
use serde_json::Value;

use crate::video::prompt_json::VideoSegmentPlan;
use crate::video::prompt_preview::VideoPromptRequestPreviewStep;
use crate::video::prompt_preview::preview_steps::{reused_step, step_from_request};

use super::super::super::requests::{
    FramePromptPreviewRequestInput, build_first_frame_prompt_preview_user,
    build_frame_prompt_preview_request,
};
use super::contracts::FrameWorkflowContext;

pub(super) async fn append_first_frame_preview_step(
    context: FrameWorkflowContext<'_, '_>,
    segment_json: &Value,
    segment_plan: &[VideoSegmentPlan],
    allowed_reference_ids: &HashSet<i64>,
    execution_identity: &Value,
    steps: &mut Vec<VideoPromptRequestPreviewStep>,
) -> Result<(), AppError> {
    if context
        .service
        .reusable_first_frame_prompts_for_preview(
            context.project_id,
            context.chapter_id,
            context.input,
            segment_plan,
            allowed_reference_ids,
            execution_identity,
            true,
        )
        .await?
        .is_none()
    {
        let user = build_first_frame_prompt_preview_user(
            context.prompt_renderer,
            context.project_id,
            context.source,
            context.input,
            segment_json,
        )
        .await?;
        let request = build_frame_prompt_preview_request(FramePromptPreviewRequestInput {
            prompt_renderer: context.prompt_renderer,
            project_id: context.project_id,
            source: context.source,
            input: context.input,
            prompt_name: context.spec.frame_prompt_name,
            user,
            segment_json,
            asset_context: &context.input.available_assets,
            segment_placeholder: false,
        })
        .await?;
        steps.push(step_from_request(
            context.spec.frame_key,
            context.spec.frame_title,
            context.spec.frame_prompt_name,
            request,
        )?);
        return Ok(());
    }

    steps.push(reused_step(
        context.spec.frame_key,
        context.spec.frame_title,
        context.spec.frame_prompt_name,
        &format!(
            "已存在完整可复用{}，本次生成会跳过该 LLM 请求",
            context.spec.frame_title
        ),
    ));
    Ok(())
}
