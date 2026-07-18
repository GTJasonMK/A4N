use std::collections::{HashMap, HashSet};

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::video::prompt_generation::frame_prompts::{
    FramePromptGenerationSpec, NewChapterVideoPrompt, VideoPromptDraftCommonInput,
};
use crate::video::prompt_json::VideoSegmentPlan;
use crate::video::{
    ChapterVideoSourceContext, OrchestratorContext, VideoPromptChunk, VideoPromptProgress,
};
use a4n_core::prompt_language::PromptOutputLanguage;
use a4n_core::source_spans::SourceSpan;
use a4n_prompt::rendering::PromptRenderer;

use super::boundary_workflow::{BoundaryFrameWorkflowInput, generate_boundary_frame_workflow};
use super::first_frame_workflow::{FirstFrameWorkflowInput, generate_first_frame_workflow};
use super::storyboard_workflow::{StoryboardWorkflowInput, generate_storyboard_workflow};
use crate::video::prompt_generation::frame_prompts::{FramePromptLayout, VideoUnitPlan};

pub(super) struct FrameLayoutDispatchInput<'a, 'source, 'progress, 'chunk, 'borrow> {
    pub service: &'a OrchestratorContext,
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub chapter_id: i64,
    pub source: ChapterVideoSourceContext<'source>,
    pub source_metadata: &'a Value,
    pub source_spans: &'a [SourceSpan],
    pub variables: &'a HashMap<String, Value>,
    pub project_context: Option<&'a Value>,
    pub available_assets: &'a Value,
    pub reference_image_ids: &'a [i64],
    pub reference_ids: &'a Value,
    pub allowed_reference_ids: &'a HashSet<i64>,
    pub segment_json: &'a Value,
    pub segment_plan: &'a [VideoSegmentPlan],
    pub video_units: &'a [VideoUnitPlan],
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub aspect_ratio: &'a str,
    pub frame_source_prompt_type: &'a str,
    pub workflow_run_id: Option<i64>,
    pub expected_frame_count: usize,
    pub prompt_batch_size: usize,
    pub regenerate_existing: bool,
    pub reusable_frame_rows: Vec<chapter_video_prompt::Model>,
    pub frame_draft_common: VideoPromptDraftCommonInput<'a>,
    pub segment_draft: Option<&'a NewChapterVideoPrompt>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub cancel_token: Option<&'a CancellationToken>,
    pub progress: &'borrow mut Option<VideoPromptProgress<'progress>>,
    pub chunks: &'borrow mut Option<VideoPromptChunk<'chunk>>,
}

pub(super) async fn generate_frame_layout(
    input: FrameLayoutDispatchInput<'_, '_, '_, '_, '_>,
) -> Result<Vec<NewChapterVideoPrompt>, AppError> {
    let reusable_frame_rows = input.reusable_frame_rows;
    if matches!(input.spec.layout, FramePromptLayout::BoundaryFrames) {
        generate_boundary_frame_workflow(BoundaryFrameWorkflowInput {
            service: input.service,
            spec: input.spec,
            project_id: input.project_id,
            chapter_id: input.chapter_id,
            source: input.source,
            source_metadata: input.source_metadata,
            source_spans: input.source_spans,
            variables: input.variables,
            project_context: input.project_context,
            available_assets: input.available_assets,
            reference_image_ids: input.reference_image_ids,
            segment_json: input.segment_json,
            segment_plan: input.segment_plan,
            style: input.style,
            prompt_output_language: input.prompt_output_language,
            aspect_ratio: input.aspect_ratio,
            frame_source_prompt_type: input.frame_source_prompt_type,
            workflow_run_id: input.workflow_run_id,
            expected_frame_count: input.expected_frame_count,
            prompt_batch_size: input.prompt_batch_size,
            regenerate_existing: input.regenerate_existing,
            reusable_rows: reusable_frame_rows,
            allowed_reference_ids: input.allowed_reference_ids,
            frame_draft_common: input.frame_draft_common,
            segment_draft: input.segment_draft,
            prompt_renderer: input.prompt_renderer,
            cancel_token: input.cancel_token,
            progress: input.progress,
            chunks: input.chunks,
        })
        .await
    } else if input.spec.is_storyboard() {
        generate_storyboard_workflow(StoryboardWorkflowInput {
            service: input.service,
            spec: input.spec,
            project_id: input.project_id,
            chapter_id: input.chapter_id,
            source: input.source,
            source_spans: input.source_spans,
            variables: input.variables,
            project_context: input.project_context,
            available_assets: input.available_assets,
            reference_image_ids: input.reference_image_ids,
            reference_ids: input.reference_ids,
            allowed_reference_ids: input.allowed_reference_ids,
            segment_json: input.segment_json,
            segment_plan: input.segment_plan,
            video_units: input.video_units,
            style: input.style,
            prompt_output_language: input.prompt_output_language,
            aspect_ratio: input.aspect_ratio,
            frame_source_prompt_type: input.frame_source_prompt_type,
            workflow_run_id: input.workflow_run_id,
            expected_frame_count: input.expected_frame_count,
            prompt_batch_size: input.prompt_batch_size,
            regenerate_existing: input.regenerate_existing,
            reusable_rows: reusable_frame_rows,
            frame_draft_common: input.frame_draft_common,
            segment_draft: input.segment_draft,
            prompt_renderer: input.prompt_renderer,
            cancel_token: input.cancel_token,
            progress: input.progress,
            chunks: input.chunks,
        })
        .await
    } else {
        generate_first_frame_workflow(FirstFrameWorkflowInput {
            service: input.service,
            spec: input.spec,
            project_id: input.project_id,
            chapter_id: input.chapter_id,
            source: input.source,
            source_spans: input.source_spans,
            variables: input.variables,
            project_context: input.project_context,
            available_assets: input.available_assets,
            reference_image_ids: input.reference_image_ids,
            reference_ids: input.reference_ids,
            allowed_reference_ids: input.allowed_reference_ids,
            segment_json: input.segment_json,
            segment_plan: input.segment_plan,
            style: input.style,
            prompt_output_language: input.prompt_output_language,
            aspect_ratio: input.aspect_ratio,
            frame_source_prompt_type: input.frame_source_prompt_type,
            workflow_run_id: input.workflow_run_id,
            expected_frame_count: input.expected_frame_count,
            regenerate_existing: input.regenerate_existing,
            frame_draft_common: input.frame_draft_common,
            segment_draft: input.segment_draft,
            prompt_renderer: input.prompt_renderer,
            cancel_token: input.cancel_token,
            progress: input.progress,
            chunks: input.chunks,
        })
        .await
    }
}
