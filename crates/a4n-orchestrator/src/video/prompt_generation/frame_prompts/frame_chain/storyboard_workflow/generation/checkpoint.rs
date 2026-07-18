mod request;
mod validation;

use std::collections::{HashMap, HashSet};

use a4n_core::error::AppError;
use a4n_core::prompt_language::PromptOutputLanguage;
use a4n_core::source_spans::SourceSpan;
use a4n_db::workflow;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::video::prompt_generation::frame_prompts::{
    FramePromptGenerationSpec, RenderOrRestoreJsonWithValidationInput, RenderedVideoPromptTrace,
    VideoUnitPlan, frame_prompt_progress_spec, render_or_restore_json_with_validation_with_context,
    serialize_video_value,
};
use crate::video::prompt_generation::{ProgressSpan, VideoPromptStepProgress};
use crate::video::prompt_json::{VideoFramePrompt, VideoSegmentPlan};
use crate::video::{
    ChapterVideoSourceContext, OrchestratorContext, VideoPromptChunk, VideoPromptProgress,
};
use crate::workflow::workflow_checkpoint_contract_app_error;

use self::request::{
    PreparedStoryboardFrameRequest, StoryboardFrameRequestInput, prepare_storyboard_frame_request,
};
use self::validation::{StoryboardFrameValidationInput, parse_and_validate_storyboard_frame};

pub(super) struct StoryboardFrameCheckpointInput<'a, 'source, 'progress, 'chunk> {
    pub service: &'a OrchestratorContext,
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub chapter_id: i64,
    pub source: ChapterVideoSourceContext<'source>,
    pub source_spans: &'a [SourceSpan],
    pub variables: &'a HashMap<String, Value>,
    pub project_context: Option<&'a Value>,
    pub frame_prompt_available_assets: &'a Value,
    pub reference_image_ids: &'a [i64],
    pub reference_ids: &'a Value,
    pub allowed_reference_ids: &'a HashSet<i64>,
    pub segment_json: &'a Value,
    pub segment_plan: &'a [VideoSegmentPlan],
    pub unit: &'a VideoUnitPlan,
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub aspect_ratio: &'a str,
    pub workflow_run_id: Option<i64>,
    pub expected_frame_count: usize,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub cancel_token: Option<&'a CancellationToken>,
    pub frame_progress_span: ProgressSpan,
    pub generation_message: &'a str,
    pub progress: &'a mut Option<VideoPromptProgress<'progress>>,
    pub chunks: &'a mut Option<VideoPromptChunk<'chunk>>,
}

pub(super) async fn render_storyboard_frame_checkpoint(
    input: StoryboardFrameCheckpointInput<'_, '_, '_, '_>,
) -> Result<(RenderedVideoPromptTrace, VideoFramePrompt), AppError> {
    let StoryboardFrameCheckpointInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        source_spans,
        variables,
        project_context,
        frame_prompt_available_assets,
        reference_image_ids,
        reference_ids,
        allowed_reference_ids,
        segment_json,
        segment_plan,
        unit,
        style,
        prompt_output_language,
        aspect_ratio,
        workflow_run_id,
        expected_frame_count,
        prompt_renderer,
        cancel_token,
        frame_progress_span,
        generation_message,
        progress,
        chunks,
    } = input;
    let prepared = prepare_storyboard_frame_request(StoryboardFrameRequestInput {
        spec,
        project_id,
        chapter_id,
        source,
        source_spans,
        variables,
        project_context,
        frame_prompt_available_assets,
        reference_ids,
        segment_json,
        segment_plan,
        unit,
        style,
        prompt_output_language,
        aspect_ratio,
        expected_frame_count,
        prompt_renderer,
    })
    .await?;
    let PreparedStoryboardFrameRequest {
        frame_number,
        duration_secs,
        required_aspect_ratio,
        expected_segment_numbers,
        context_manifest,
        step_key,
        fingerprint_json,
    } = prepared;
    let frame_workflow_run_id =
        workflow::required_llm_call_workflow_run_id(workflow_run_id, &step_key)
            .map_err(workflow_checkpoint_contract_app_error)?;

    render_or_restore_json_with_validation_with_context(
        RenderOrRestoreJsonWithValidationInput {
            service,
            workflow_run_id: frame_workflow_run_id,
            call_key: step_key,
            call_order: 300 + frame_number,
            input_fingerprint_json: fingerprint_json,
            prompt_renderer,
            prompt_name: spec.frame_prompt_name,
            project_id,
            variables,
            context_manifest,
            temperature: 0.45,
            cancel_token,
            step: VideoPromptStepProgress::new(
                frame_progress_span.start(),
                100,
                generation_message,
            )
            .with_span(frame_progress_span),
            progress,
            chunks,
            structured_progress: Some(frame_prompt_progress_spec("分镜", Some(1))),
            checkpoint_context: "分镜提示词断点",
        },
        |raw| {
            parse_and_validate_storyboard_frame(StoryboardFrameValidationInput {
                raw,
                frame_number,
                duration_secs,
                default_aspect_ratio: aspect_ratio,
                required_aspect_ratio: &required_aspect_ratio,
                expected_segment_numbers: &expected_segment_numbers,
                reference_image_ids,
                allowed_reference_ids,
                style,
                prompt_output_language,
            })
        },
        |frame_prompt| serialize_video_value(frame_prompt, "分镜提示词断点"),
    )
    .await
}
