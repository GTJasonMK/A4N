use std::collections::HashMap;

use serde_json::json;

use a4n_core::error::AppError;

use crate::video::prompt_context::{
    FinalVideoPromptContext, build_boundary_video_prompt_user, build_chat_video_prompt_user,
    build_first_frame_video_prompt_user, build_storyboard_video_prompt_user,
};
use crate::video::prompt_generation::{
    reusable_boundary_video_prompts_from_rows, reusable_chat_video_prompts_from_rows,
    reusable_first_frame_video_prompts_from_rows, video_prompt_segments_with_reference_ids,
};
use crate::video::prompt_records::{PromptWorkflowRowsContext, VideoExecutionIdentityMatch};
use crate::video::{ChapterVideoSourceContext, OrchestratorContext};
use a4n_prompt::rendering::PromptRenderer;

use super::inputs::PreparedVideoPromptPreviewInputs;
use super::planning::frame_prompts_from_preview_context;
use super::requests::{
    VideoPromptPreviewRequestInput, build_video_prompt_preview_request,
    push_video_prompt_request_step,
};
use super::spec::{
    LocalVideoPromptPreviewMode, LocalVideoPromptPreviewSpec, VideoPromptPreviewTarget,
    prompt_values_by_shot, video_prompt_target,
};
use super::video_predecessors::{ResolvedVideoPromptPredecessors, SegmentPreviewSource};
use super::{
    VideoPromptRequestPreview, VideoPromptRequestPreviewStep, preview_chat_video_prompt_from_steps,
    preview_storyboard_video_prompt_from_steps, preview_video_prompt_from_steps, reused_step,
};

pub(super) struct CompleteVideoPromptPreviewInput<'a, 'source> {
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) source: &'a ChapterVideoSourceContext<'source>,
    pub(super) prepared: &'a PreparedVideoPromptPreviewInputs,
    pub(super) spec: LocalVideoPromptPreviewSpec,
    pub(super) predecessors: ResolvedVideoPromptPredecessors,
}

impl OrchestratorContext {
    pub(super) async fn complete_video_prompt_preview(
        &self,
        mut steps: Vec<VideoPromptRequestPreviewStep>,
        request: CompleteVideoPromptPreviewInput<'_, '_>,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        let CompleteVideoPromptPreviewInput {
            prompt_renderer,
            project_id,
            chapter_id,
            source,
            prepared,
            spec,
            predecessors,
        } = request;
        let ResolvedVideoPromptPredecessors {
            segment_plan,
            frame_json,
            segment_source,
            frame_is_placeholder,
        } = predecessors;
        let segment_is_placeholder = segment_source.is_placeholder();
        let frame_prompts_for_reference = frame_prompts_from_preview_context(&frame_json)?;
        let frame_by_number = frame_prompts_for_reference
            .iter()
            .map(|frame| (frame.frame_number, frame.clone()))
            .collect::<HashMap<_, _>>();
        let video_segment_plan = video_prompt_segments_with_reference_ids(
            spec.prompt_workflow_mode(),
            &segment_plan,
            &frame_by_number,
            &HashMap::new(),
            &prepared.reference_image_ids,
        );
        let video_segment_json = json!({ "segments": &video_segment_plan });
        let matching_video_prompts = match &segment_source {
            SegmentPreviewSource::Placeholder => Vec::new(),
            SegmentPreviewSource::Reusable { execution_identity } => {
                self.matching_prompt_rows_for_workflow(
                    PromptWorkflowRowsContext {
                        project_id,
                        chapter_id,
                        workflow_mode: spec.workflow_mode(),
                        prompt_type: "video_prompt",
                        source_metadata: &prepared.source_metadata,
                        style: &prepared.style,
                        duration_secs: prepared.duration_secs,
                        aspect_ratio: &prepared.aspect_ratio,
                        reference_ids: &prepared.reference_ids,
                        generation_asset_fingerprint: Some(&prepared.generation_asset_fingerprint),
                        source_prompt_type: Some(spec.segment_prompt_type),
                        video_execution_identity: VideoExecutionIdentityMatch::Base(
                            execution_identity,
                        ),
                    },
                    prepared.prompt_output_language,
                )
                .await?
            }
        };
        let reusable_video_prompts = if prepared.regenerate_existing {
            Default::default()
        } else {
            match spec.mode {
                LocalVideoPromptPreviewMode::BoundaryFrames => prompt_values_by_shot(
                    reusable_boundary_video_prompts_from_rows(&matching_video_prompts)?,
                )?,
                LocalVideoPromptPreviewMode::FirstFrameChain => prompt_values_by_shot(
                    reusable_first_frame_video_prompts_from_rows(&matching_video_prompts)?,
                )?,
                LocalVideoPromptPreviewMode::ChatVideo => prompt_values_by_shot(
                    reusable_chat_video_prompts_from_rows(&matching_video_prompts)?,
                )?,
                LocalVideoPromptPreviewMode::Storyboard => prompt_values_by_shot(
                    reusable_boundary_video_prompts_from_rows(&matching_video_prompts)?,
                )?,
            }
        };
        let target = video_prompt_target(
            &video_segment_plan,
            &reusable_video_prompts,
            prepared.regenerate_existing,
            spec.reuse_sequence,
        );
        let (current_segment, previous_shot) = match target {
            VideoPromptPreviewTarget::Empty => {
                steps.push(reused_step(
                    "video_prompt",
                    spec.video_title,
                    spec.video_prompt_name,
                    &format!("片段规划为空，本次不会调用{}生成模型", spec.video_title),
                ));
                return Ok(preview_video_prompt_from_steps(steps));
            }
            VideoPromptPreviewTarget::Complete => {
                steps.push(reused_step(
                    "video_prompt",
                    spec.video_title,
                    spec.video_prompt_name,
                    &format!("所有片段均已有{}，本次生成不会调用模型", spec.video_title),
                ));
                return Ok(preview_video_prompt_from_steps(steps));
            }
            VideoPromptPreviewTarget::Pending {
                current_segment,
                previous_shot,
            } => (current_segment, previous_shot),
        };
        let current_segment_json = json!(current_segment);
        let previous_shot_json = previous_shot.cloned();
        let final_prompt_context = FinalVideoPromptContext {
            prompt_renderer,
            project_id,
            chapter_summary: source.summary,
            chapter_analysis: source.analysis,
            segment_plan: &video_segment_json,
            frame_prompts: &frame_json,
            current_segment: &current_segment_json,
            previous_completed_shot: previous_shot_json.as_ref(),
            available_assets: &prepared.available_assets,
            style: &prepared.style,
            prompt_output_language: prepared.prompt_output_language,
            duration_secs: prepared.duration_secs,
            aspect_ratio: &prepared.aspect_ratio,
            reference_ids: &prepared.reference_ids,
        };
        let user = match spec.mode {
            LocalVideoPromptPreviewMode::BoundaryFrames => {
                build_boundary_video_prompt_user(final_prompt_context).await
            }
            LocalVideoPromptPreviewMode::FirstFrameChain => {
                build_first_frame_video_prompt_user(final_prompt_context).await
            }
            LocalVideoPromptPreviewMode::ChatVideo => {
                build_chat_video_prompt_user(final_prompt_context).await
            }
            LocalVideoPromptPreviewMode::Storyboard => {
                build_storyboard_video_prompt_user(final_prompt_context).await
            }
        }?;
        let draft_request = build_video_prompt_preview_request(VideoPromptPreviewRequestInput {
            prompt_renderer,
            project_id,
            source,
            input: prepared,
            prompt_name: spec.video_prompt_name,
            frame_prompt_name: spec.frame_prompt_name,
            user,
        })
        .await?;
        push_video_prompt_request_step(
            &mut steps,
            spec.video_title,
            spec.video_prompt_name,
            draft_request,
            (segment_is_placeholder || frame_is_placeholder).then_some(spec.video_placeholder_note),
            None,
        )?;
        Ok(match spec.mode {
            LocalVideoPromptPreviewMode::ChatVideo => preview_chat_video_prompt_from_steps(steps),
            LocalVideoPromptPreviewMode::Storyboard => {
                preview_storyboard_video_prompt_from_steps(steps)
            }
            _ => preview_video_prompt_from_steps(steps),
        })
    }
}
