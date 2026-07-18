use a4n_core::error::AppError;

use crate::video::OrchestratorContext;
use crate::video::prompt_generation::{
    ReusableSegmentPlanForPreviewQuery, reusable_segment_plan_for_preview,
};
use crate::video::prompt_preview::VideoPromptRequestPreview;

use super::super::inputs::LocalPromptPreviewInput;
use super::super::spec::LocalVideoPromptPreviewSpec;

mod boundary;
mod contracts;
mod first_frame;
mod placeholder;
mod projection;
mod reusable;
mod storyboard;

use self::contracts::FrameWorkflowContext;
use self::placeholder::build_placeholder_frame_preview;
use self::reusable::build_reusable_frame_preview;

impl OrchestratorContext {
    pub(in crate::video::prompt_preview::local) async fn preview_local_frame_prompt_requests(
        &self,
        request: LocalPromptPreviewInput<'_, '_>,
        spec: LocalVideoPromptPreviewSpec,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        let LocalPromptPreviewInput {
            project_id,
            chapter_id,
            source,
            prompt_renderer,
            variables,
            options,
        } = request;
        let input = self
            .prepare_video_prompt_preview_inputs(
                project_id, chapter_id, &source, variables, options, true,
            )
            .await?;
        let context = FrameWorkflowContext {
            service: self,
            spec,
            project_id,
            chapter_id,
            source: &source,
            prompt_renderer,
            input: &input,
        };
        let reusable_segment =
            reusable_segment_plan_for_preview(ReusableSegmentPlanForPreviewQuery {
                service: self,
                project_id,
                chapter_id,
                workflow_mode: spec.workflow_mode(),
                source_metadata: &input.source_metadata,
                source_spans: &input.source_spans,
                style: &input.style,
                duration_secs: input.duration_secs,
                aspect_ratio: &input.aspect_ratio,
            })
            .await?;
        match reusable_segment {
            Some(reusable_segment) => {
                build_reusable_frame_preview(context, &reusable_segment).await
            }
            None => build_placeholder_frame_preview(context).await,
        }
    }
}
