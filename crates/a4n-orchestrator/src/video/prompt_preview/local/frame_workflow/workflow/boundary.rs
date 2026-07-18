use std::collections::HashSet;

use a4n_core::error::AppError;
use serde_json::Value;

use crate::video::prompt_json::VideoSegmentPlan;
use crate::video::prompt_preview::VideoPromptRequestPreviewStep;
use crate::video::prompt_preview::preview_steps::{reused_step, step_from_request};

use super::super::super::requests::{
    FramePromptPreviewRequestInput, build_boundary_frame_prompt_preview_user,
    build_frame_prompt_preview_request,
};
use super::super::super::states::{
    BoundaryFramePromptPreviewState, BoundaryFramePromptPreviewStateInput,
};
use super::contracts::FrameWorkflowContext;

pub(super) async fn append_boundary_frame_preview_step(
    context: FrameWorkflowContext<'_, '_>,
    segment_json: &Value,
    segment_plan: &[VideoSegmentPlan],
    allowed_reference_ids: &HashSet<i64>,
    execution_identity: &Value,
    steps: &mut Vec<VideoPromptRequestPreviewStep>,
) -> Result<(), AppError> {
    match context
        .service
        .boundary_frame_prompt_preview_state(BoundaryFramePromptPreviewStateInput {
            project_id: context.project_id,
            chapter_id: context.chapter_id,
            input: context.input,
            segment_plan,
            allowed_reference_ids,
            workflow_mode: context.spec.prompt_workflow_mode(),
            frame_prompt_type: context.spec.frame_prompt_type,
            frame_source_prompt_type: context.spec.frame_source_prompt_type,
            execution_identity,
        })
        .await?
    {
        BoundaryFramePromptPreviewState::Complete(_) => {
            steps.push(reused_step(
                context.spec.frame_key,
                context.spec.frame_title,
                context.spec.frame_prompt_name,
                &format!(
                    "已存在完整可复用{}，本次生成会跳过该 LLM 请求",
                    context.spec.frame_title
                ),
            ));
        }
        BoundaryFramePromptPreviewState::Next(next) => {
            let user = build_boundary_frame_prompt_preview_user(
                context.prompt_renderer,
                context.project_id,
                context.source,
                context.input,
                segment_json,
                &next.current_frame,
                &next.available_assets,
                &next.reference_ids,
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
                asset_context: &next.available_assets,
                segment_placeholder: false,
            })
            .await?;
            steps.push(step_from_request(
                context.spec.frame_key,
                context.spec.frame_title,
                context.spec.frame_prompt_name,
                request,
            )?);
        }
    }
    Ok(())
}
