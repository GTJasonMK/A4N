use std::collections::HashSet;

use a4n_core::error::AppError;
use serde_json::{Value, json};

use crate::video::prompt_json::VideoSegmentPlan;
use crate::video::{ChapterVideoSourceContext, OrchestratorContext};
use a4n_prompt::rendering::PromptRenderer;

use super::super::inputs::PreparedVideoPromptPreviewInputs;
use super::super::requests::{
    FramePromptPreviewRequestInput, build_boundary_frame_prompt_preview_user,
    build_first_frame_prompt_preview_user, build_frame_prompt_preview_request,
    build_storyboard_prompt_preview_user,
};
use super::super::spec::{LocalVideoPromptPreviewMode, LocalVideoPromptPreviewSpec};
use super::super::states::{
    BoundaryFramePromptPreviewState, BoundaryFramePromptPreviewStateInput,
    StoryboardPromptPreviewState,
};
use super::super::{
    VideoPromptRequestPreviewStep, placeholder_boundary_frame_context_for_video_prompt,
    placeholder_first_frame_prompts, placeholder_storyboard_prompts, reused_step,
    step_from_request,
};

pub(super) struct ResolveAvailableVideoFramePredecessorInput<'a, 'source> {
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) source: &'a ChapterVideoSourceContext<'source>,
    pub(super) prepared: &'a PreparedVideoPromptPreviewInputs,
    pub(super) spec: LocalVideoPromptPreviewSpec,
    pub(super) segment_json: &'a Value,
    pub(super) segment_plan: &'a [VideoSegmentPlan],
    pub(super) allowed_reference_ids: &'a HashSet<i64>,
    pub(super) execution_identity: &'a Value,
}

impl OrchestratorContext {
    pub(super) async fn resolve_available_video_frame_predecessor(
        &self,
        steps: &mut Vec<VideoPromptRequestPreviewStep>,
        request: ResolveAvailableVideoFramePredecessorInput<'_, '_>,
    ) -> Result<(Value, bool), AppError> {
        let ResolveAvailableVideoFramePredecessorInput {
            prompt_renderer,
            project_id,
            chapter_id,
            source,
            prepared,
            spec,
            segment_json,
            segment_plan,
            allowed_reference_ids,
            execution_identity,
        } = request;
        match spec.mode {
            LocalVideoPromptPreviewMode::BoundaryFrames
            | LocalVideoPromptPreviewMode::ChatVideoBoundaryFrames => {
                match self
                    .boundary_frame_prompt_preview_state(BoundaryFramePromptPreviewStateInput {
                        project_id,
                        chapter_id,
                        input: prepared,
                        segment_plan,
                        allowed_reference_ids,
                        workflow_mode: spec.prompt_workflow_mode(),
                        frame_prompt_type: spec.frame_prompt_type,
                        frame_source_prompt_type: spec.frame_source_prompt_type,
                        execution_identity,
                    })
                    .await?
                {
                    BoundaryFramePromptPreviewState::Complete(frame_prompts) => {
                        steps.push(reused_frame_step(spec));
                        Ok((json!({ "frames": frame_prompts }), false))
                    }
                    BoundaryFramePromptPreviewState::Next(next) => {
                        let frame_user = build_boundary_frame_prompt_preview_user(
                            prompt_renderer,
                            project_id,
                            source,
                            prepared,
                            segment_json,
                            &next.current_frame,
                            &next.available_assets,
                            &next.reference_ids,
                        )
                        .await?;
                        let frame_request =
                            build_frame_prompt_preview_request(FramePromptPreviewRequestInput {
                                prompt_renderer,
                                project_id,
                                source,
                                input: prepared,
                                prompt_name: spec.frame_prompt_name,
                                user: frame_user,
                                segment_json,
                                asset_context: &next.available_assets,
                                segment_placeholder: false,
                            })
                            .await?;
                        steps.push(step_from_request(
                            spec.frame_key,
                            spec.frame_title,
                            spec.frame_prompt_name,
                            frame_request,
                        )?);
                        Ok((
                            placeholder_boundary_frame_context_for_video_prompt(segment_plan)?,
                            true,
                        ))
                    }
                }
            }
            LocalVideoPromptPreviewMode::FirstFrameChain => {
                if let Some(frame_prompts) = self
                    .reusable_first_frame_prompts_for_preview(
                        project_id,
                        chapter_id,
                        prepared,
                        segment_plan,
                        allowed_reference_ids,
                        execution_identity,
                        true,
                    )
                    .await?
                {
                    steps.push(reused_frame_step(spec));
                    Ok((json!({ "frames": frame_prompts }), false))
                } else {
                    let frame_user = build_first_frame_prompt_preview_user(
                        prompt_renderer,
                        project_id,
                        source,
                        prepared,
                        segment_json,
                    )
                    .await?;
                    let frame_request =
                        build_frame_prompt_preview_request(FramePromptPreviewRequestInput {
                            prompt_renderer,
                            project_id,
                            source,
                            input: prepared,
                            prompt_name: spec.frame_prompt_name,
                            user: frame_user,
                            segment_json,
                            asset_context: &prepared.available_assets,
                            segment_placeholder: false,
                        })
                        .await?;
                    steps.push(step_from_request(
                        spec.frame_key,
                        spec.frame_title,
                        spec.frame_prompt_name,
                        frame_request,
                    )?);
                    Ok((placeholder_first_frame_prompts(segment_plan), true))
                }
            }
            LocalVideoPromptPreviewMode::Storyboard => {
                match self
                    .storyboard_prompt_preview_state(
                        project_id,
                        chapter_id,
                        prepared,
                        segment_plan,
                        allowed_reference_ids,
                        execution_identity,
                    )
                    .await?
                {
                    StoryboardPromptPreviewState::Complete(frame_prompts) => {
                        steps.push(reused_frame_step(spec));
                        Ok((json!({ "frames": frame_prompts }), false))
                    }
                    StoryboardPromptPreviewState::Next(next) => {
                        let frame_user = build_storyboard_prompt_preview_user(
                            prompt_renderer,
                            project_id,
                            source,
                            prepared,
                            segment_json,
                            &next.current_frame,
                            &next.available_assets,
                        )
                        .await?;
                        let frame_request =
                            build_frame_prompt_preview_request(FramePromptPreviewRequestInput {
                                prompt_renderer,
                                project_id,
                                source,
                                input: prepared,
                                prompt_name: spec.frame_prompt_name,
                                user: frame_user,
                                segment_json,
                                asset_context: &next.available_assets,
                                segment_placeholder: false,
                            })
                            .await?;
                        steps.push(step_from_request(
                            spec.frame_key,
                            spec.frame_title,
                            spec.frame_prompt_name,
                            frame_request,
                        )?);
                        Ok((placeholder_storyboard_prompts(segment_plan)?, true))
                    }
                }
            }
        }
    }
}

fn reused_frame_step(spec: LocalVideoPromptPreviewSpec) -> VideoPromptRequestPreviewStep {
    reused_step(
        spec.frame_key,
        spec.frame_title,
        spec.frame_prompt_name,
        &format!(
            "已存在完整{}，视频提示词请求会直接使用该前置结果",
            spec.frame_title
        ),
    )
}
