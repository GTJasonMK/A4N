use a4n_core::error::AppError;

use crate::video::prompt_generation::{
    ReusableSegmentPlanForPreviewQuery, reusable_segment_plan_for_preview,
};
use crate::video::{ChapterVideoSourceContext, OrchestratorContext};
use a4n_prompt::rendering::PromptRenderer;

use super::super::inputs::{PreparedVideoPromptPreviewInputs, allowed_reference_ids_for_preview};
use super::super::planning::SegmentPlanPreviewStepsInput;
use super::super::requests::{
    FramePromptPreviewRequestInput, build_boundary_frame_prompt_preview_user,
    build_first_frame_prompt_preview_user, build_frame_prompt_preview_request,
    build_storyboard_prompt_preview_user,
};
use super::super::spec::{
    LocalFramePromptPreviewMode, LocalVideoPromptPreviewMode, LocalVideoPromptPreviewSpec,
};
use super::super::{
    VideoPromptRequestPreviewStep, placeholder_boundary_frame_context_for_video_prompt,
    placeholder_first_frame_prompts, placeholder_segment_plan, placeholder_segments,
    placeholder_step_from_request, placeholder_storyboard_prompts, reused_step,
};
use super::frames::ResolveAvailableVideoFramePredecessorInput;
use super::{ResolvedVideoPromptPredecessors, SegmentPreviewSource};

pub(in crate::video::prompt_preview::local) struct ResolveVideoPromptPredecessorsInput<'a, 'source>
{
    pub(in crate::video::prompt_preview::local) prompt_renderer: &'a dyn PromptRenderer,
    pub(in crate::video::prompt_preview::local) project_id: i64,
    pub(in crate::video::prompt_preview::local) chapter_id: i64,
    pub(in crate::video::prompt_preview::local) source: &'a ChapterVideoSourceContext<'source>,
    pub(in crate::video::prompt_preview::local) prepared: &'a PreparedVideoPromptPreviewInputs,
    pub(in crate::video::prompt_preview::local) spec: LocalVideoPromptPreviewSpec,
}

impl OrchestratorContext {
    pub(in crate::video::prompt_preview::local) async fn resolve_video_prompt_predecessors(
        &self,
        steps: &mut Vec<VideoPromptRequestPreviewStep>,
        request: ResolveVideoPromptPredecessorsInput<'_, '_>,
    ) -> Result<ResolvedVideoPromptPredecessors, AppError> {
        let ResolveVideoPromptPredecessorsInput {
            prompt_renderer,
            project_id,
            chapter_id,
            source,
            prepared,
            spec,
        } = request;
        let reusable_segment =
            reusable_segment_plan_for_preview(ReusableSegmentPlanForPreviewQuery {
                service: self,
                project_id,
                chapter_id,
                workflow_mode: spec.workflow_mode(),
                source_metadata: &prepared.source_metadata,
                source_spans: &prepared.source_spans,
                style: &prepared.style,
                duration_secs: prepared.duration_secs,
                aspect_ratio: &prepared.aspect_ratio,
            })
            .await?;
        let (segment_json, segment_plan, segment_source) =
            if let Some(reusable_segment) = reusable_segment {
                steps.push(reused_step(
                    "segment_plan",
                    spec.segment_title,
                    "chapter_video_segment_plan",
                    spec.segment_reused_note,
                ));
                (
                    reusable_segment.segment_json,
                    reusable_segment.segment_plan,
                    SegmentPreviewSource::Reusable {
                        execution_identity: reusable_segment.execution_identity,
                    },
                )
            } else {
                let project_context = self
                    .chapter_video_project_context(project_id, source.analysis)
                    .await?;
                self.push_segment_plan_preview_steps(
                    steps,
                    SegmentPlanPreviewStepsInput {
                        prompt_renderer,
                        project_id,
                        chapter_id,
                        source,
                        input: prepared,
                        project_context: project_context.as_ref(),
                        segment_title: spec.segment_title,
                    },
                )
                .await?;
                (
                    placeholder_segment_plan(),
                    placeholder_segments(),
                    SegmentPreviewSource::Placeholder,
                )
            };
        let allowed_reference_ids = allowed_reference_ids_for_preview(
            &prepared.available_assets,
            &prepared.reference_image_ids,
        )?;

        let (frame_json, frame_is_placeholder) = if spec.mode
            == LocalVideoPromptPreviewMode::ChatVideo
        {
            (serde_json::json!({ "frames": [] }), false)
        } else {
            let frame_mode = spec.frame_mode().ok_or_else(|| {
                AppError::InvalidArgument("当前视频提示词预览缺少帧工作流模式".to_string())
            })?;
            match &segment_source {
                SegmentPreviewSource::Placeholder => {
                    let frame_user = match frame_mode {
                        LocalFramePromptPreviewMode::BoundaryFrames => {
                            let next = Self::placeholder_boundary_frame_prompt_preview_state(
                                prepared,
                                &segment_plan,
                            )?;
                            build_boundary_frame_prompt_preview_user(
                                prompt_renderer,
                                project_id,
                                source,
                                prepared,
                                &segment_json,
                                &next.current_frame,
                                &next.available_assets,
                                &next.reference_ids,
                            )
                            .await?
                        }
                        LocalFramePromptPreviewMode::FirstFrameChain => {
                            build_first_frame_prompt_preview_user(
                                prompt_renderer,
                                project_id,
                                source,
                                prepared,
                                &segment_json,
                            )
                            .await?
                        }
                        LocalFramePromptPreviewMode::Storyboard => {
                            let next = Self::placeholder_storyboard_prompt_preview_state(
                                prepared,
                                &segment_plan,
                            )?;
                            build_storyboard_prompt_preview_user(
                                prompt_renderer,
                                project_id,
                                source,
                                prepared,
                                &segment_json,
                                &next.current_frame,
                                &next.available_assets,
                            )
                            .await?
                        }
                    };
                    let frame_request =
                        build_frame_prompt_preview_request(FramePromptPreviewRequestInput {
                            prompt_renderer,
                            project_id,
                            source,
                            input: prepared,
                            prompt_name: spec.frame_prompt_name,
                            user: frame_user,
                            segment_json: &segment_json,
                            asset_context: &prepared.available_assets,
                            segment_placeholder: true,
                        })
                        .await?;
                    steps.push(placeholder_step_from_request(
                    spec.frame_key,
                    spec.frame_title,
                    spec.frame_prompt_name,
                    frame_request,
                    "结构预览：真实生成时 previous_steps.chapter_video_segment_plan 会替换为上一步片段规划模型输出。",
                )?);
                    let frame_json = match frame_mode {
                        LocalFramePromptPreviewMode::BoundaryFrames => {
                            placeholder_boundary_frame_context_for_video_prompt(&segment_plan)?
                        }
                        LocalFramePromptPreviewMode::FirstFrameChain => {
                            placeholder_first_frame_prompts(&segment_plan)
                        }
                        LocalFramePromptPreviewMode::Storyboard => {
                            placeholder_storyboard_prompts(&segment_plan)?
                        }
                    };
                    (frame_json, true)
                }
                SegmentPreviewSource::Reusable { execution_identity } => {
                    self.resolve_available_video_frame_predecessor(
                        steps,
                        ResolveAvailableVideoFramePredecessorInput {
                            prompt_renderer,
                            project_id,
                            chapter_id,
                            source,
                            prepared,
                            spec,
                            mode: frame_mode,
                            segment_json: &segment_json,
                            segment_plan: &segment_plan,
                            allowed_reference_ids: &allowed_reference_ids,
                            execution_identity,
                        },
                    )
                    .await?
                }
            }
        };
        Ok(ResolvedVideoPromptPredecessors {
            segment_plan,
            frame_json,
            segment_source,
            frame_is_placeholder,
        })
    }
}
