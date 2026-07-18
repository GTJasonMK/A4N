use serde_json::Value;

use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;

use super::{
    ChapterVideoSourceContext, FramePromptContext, PreparedVideoPromptPreviewInputs,
    PromptRenderer, VIDEO_PREVIOUS_OUTPUT_SEGMENT_PLAN, VideoContextManifestInput,
    VideoContextPreviousOutput, VideoPromptRequestPreviewStep, build_final_video_context_manifest,
    build_first_frame_prompt_user, build_single_frame_prompt_user, build_storyboard_prompt_user,
    build_video_context_manifest, build_video_prompt_request,
    frame_prompt_available_assets_context, placeholder_step_from_request, step_from_request,
};

pub(super) async fn build_first_frame_prompt_preview_user(
    prompt_renderer: &dyn PromptRenderer,
    project_id: i64,
    source: &ChapterVideoSourceContext<'_>,
    input: &PreparedVideoPromptPreviewInputs,
    segment_json: &Value,
) -> Result<String, AppError> {
    let available_assets = frame_prompt_available_assets_context(&input.available_assets)?;
    let context = FramePromptContext {
        prompt_renderer,
        project_id,
        chapter_summary: source.summary,
        chapter_analysis: source.analysis,
        segment_plan: segment_json,
        available_assets: &available_assets,
        style: &input.style,
        prompt_output_language: input.prompt_output_language,
        aspect_ratio: &input.aspect_ratio,
        reference_ids: &input.reference_ids,
    };
    build_first_frame_prompt_user(context, source.content).await
}

pub(super) fn push_video_prompt_request_step(
    steps: &mut Vec<VideoPromptRequestPreviewStep>,
    title: &str,
    prompt_name: &str,
    request: ChatRequest,
    placeholder_note: Option<&str>,
    ready_note: Option<&str>,
) -> Result<(), AppError> {
    let mut step = if let Some(note) = placeholder_note {
        placeholder_step_from_request("video_prompt", title, prompt_name, request, note)?
    } else {
        step_from_request("video_prompt", title, prompt_name, request)?
    };
    if placeholder_note.is_none() {
        step.note = ready_note.map(str::to_string);
    }
    steps.push(step);
    Ok(())
}

pub(super) async fn build_boundary_frame_prompt_preview_user(
    prompt_renderer: &dyn PromptRenderer,
    project_id: i64,
    source: &ChapterVideoSourceContext<'_>,
    input: &PreparedVideoPromptPreviewInputs,
    segment_json: &Value,
    current_frame: &Value,
    available_assets: &Value,
    reference_ids: &Value,
) -> Result<String, AppError> {
    let context = FramePromptContext {
        prompt_renderer,
        project_id,
        chapter_summary: source.summary,
        chapter_analysis: source.analysis,
        segment_plan: segment_json,
        available_assets,
        style: &input.style,
        prompt_output_language: input.prompt_output_language,
        aspect_ratio: &input.aspect_ratio,
        reference_ids,
    };
    build_single_frame_prompt_user(context, current_frame).await
}

pub(super) async fn build_storyboard_prompt_preview_user(
    prompt_renderer: &dyn PromptRenderer,
    project_id: i64,
    source: &ChapterVideoSourceContext<'_>,
    input: &PreparedVideoPromptPreviewInputs,
    segment_json: &Value,
    current_frame: &Value,
    available_assets: &Value,
) -> Result<String, AppError> {
    let context = FramePromptContext {
        prompt_renderer,
        project_id,
        chapter_summary: source.summary,
        chapter_analysis: source.analysis,
        segment_plan: segment_json,
        available_assets,
        style: &input.style,
        prompt_output_language: input.prompt_output_language,
        aspect_ratio: &input.aspect_ratio,
        reference_ids: &input.reference_ids,
    };
    build_storyboard_prompt_user(context, current_frame).await
}

pub(super) struct FramePromptPreviewRequestInput<'a, 'source> {
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) project_id: i64,
    pub(super) source: &'a ChapterVideoSourceContext<'source>,
    pub(super) input: &'a PreparedVideoPromptPreviewInputs,
    pub(super) prompt_name: &'a str,
    pub(super) user: String,
    pub(super) segment_json: &'a Value,
    pub(super) asset_context: &'a Value,
    pub(super) segment_placeholder: bool,
}

pub(super) async fn build_frame_prompt_preview_request(
    input: FramePromptPreviewRequestInput<'_, '_>,
) -> Result<ChatRequest, AppError> {
    let FramePromptPreviewRequestInput {
        prompt_renderer,
        project_id,
        source,
        input,
        prompt_name,
        user,
        segment_json,
        asset_context,
        segment_placeholder,
    } = input;
    let context_manifest = build_video_context_manifest(VideoContextManifestInput {
        prompt_renderer,
        project_id,
        variables: &input.vars,
        source,
        source_spans: &input.source_spans,
        prompt_names: &["chapter_video_segment_plan", prompt_name],
        previous_outputs: &[VideoContextPreviousOutput {
            pack_kind: VIDEO_PREVIOUS_OUTPUT_SEGMENT_PLAN,
            source_name: "chapter_video_segment_plan",
            parsed_output: segment_json,
            raw_output: None,
            placeholder: segment_placeholder,
        }],
        project_context: input.project_context.as_ref(),
        asset_context: Some(asset_context),
        dynamic_user_content: user.clone(),
    })
    .await?;
    build_video_prompt_request(
        prompt_renderer,
        prompt_name,
        project_id,
        &input.vars,
        context_manifest,
        0.45,
    )
    .await
}

pub(super) struct VideoPromptPreviewRequestInput<'a, 'source> {
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) project_id: i64,
    pub(super) source: &'a ChapterVideoSourceContext<'source>,
    pub(super) input: &'a PreparedVideoPromptPreviewInputs,
    pub(super) prompt_name: &'a str,
    pub(super) frame_prompt_name: &'a str,
    pub(super) user: String,
}

pub(super) async fn build_video_prompt_preview_request(
    input: VideoPromptPreviewRequestInput<'_, '_>,
) -> Result<ChatRequest, AppError> {
    let VideoPromptPreviewRequestInput {
        prompt_renderer,
        project_id,
        source,
        input,
        prompt_name,
        frame_prompt_name,
        user,
    } = input;
    let context_manifest = build_final_video_context_manifest(
        prompt_renderer,
        project_id,
        &input.vars,
        source,
        &["chapter_video_segment_plan", frame_prompt_name, prompt_name],
        user.clone(),
    )
    .await?;
    build_video_prompt_request(
        prompt_renderer,
        prompt_name,
        project_id,
        &input.vars,
        context_manifest,
        0.45,
    )
    .await
}
