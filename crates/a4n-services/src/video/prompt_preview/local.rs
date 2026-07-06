use std::collections::{BTreeMap, HashMap, HashSet};

use a4n_core::error::AppError;
use a4n_engine::llm::provider::ChatRequest;
use a4n_engine::prompt::engine::PromptEngine;
use serde::Serialize;
use serde_json::{Value, json};

use crate::chapter::source_spans::{SourceSpan, source_spans_for_content};

use super::preview_steps::{
    first_cinematic_shot_for_preview, placeholder_boundary_frame_context_for_video_prompt,
    placeholder_extracted_beats, placeholder_first_frame_prompts, placeholder_segment_plan,
    placeholder_segments, placeholder_shot_analysis, placeholder_step_from_request,
    placeholder_storyboard_prompts, placeholder_video_prompt_draft, preview_boundary_from_steps,
    preview_chat_boundary_from_steps, preview_chat_video_prompt_from_steps,
    preview_first_frame_from_steps, preview_storyboard_from_steps,
    preview_storyboard_video_prompt_from_steps, preview_video_prompt_from_steps, reused_step,
    step_from_request,
};
use super::{VideoPromptRequestPreview, VideoPromptRequestPreviewStep};
use crate::video::library::available_video_image_ids;
use crate::video::prompt_context::{
    FinalVideoPromptContext, FramePromptContext, VIDEO_PREVIOUS_OUTPUT_BEAT_EXTRACTION,
    VIDEO_PREVIOUS_OUTPUT_SEGMENT_PLAN, VIDEO_PREVIOUS_OUTPUT_SHOT_ANALYSIS,
    VideoContextManifestInput, VideoContextPreviousOutput, VideoPlanningPromptContext,
    build_beat_extraction_user, build_boundary_video_prompt_review_user,
    build_boundary_video_prompt_user, build_chat_video_prompt_review_user,
    build_chat_video_prompt_user, build_final_video_context_manifest,
    build_first_frame_prompt_user, build_first_frame_video_prompt_review_user,
    build_first_frame_video_prompt_user, build_segment_plan_for_shot_user,
    build_shot_grouping_user, build_single_frame_prompt_user, build_storyboard_prompt_user,
    build_storyboard_video_prompt_review_user, build_storyboard_video_prompt_user,
    build_video_context_manifest, validate_prompt_options, video_source_metadata,
};
use crate::video::prompt_generation::{
    FrameImageLookupContext, FrameImageLookupRowsQuery,
    available_assets_with_previous_boundary_frame, boundary_frame_generation_context,
    boundary_frame_image_ids_by_number, boundary_frame_prompt_rows_prefix,
    boundary_frame_rows_for_image_lookup, build_video_prompt_request, expected_video_unit_count,
    first_frame_prompts_from_rows, frame_prompt_available_assets_context, frame_prompts_from_rows,
    initial_first_frame_rows, previous_boundary_frame_asset_context,
    reusable_boundary_video_prompts_from_rows, reusable_chat_video_prompts_from_rows,
    reusable_first_frame_video_prompts_from_rows, reusable_frame_prefix_len_with_previous_images,
    source_span_windows, storyboard_expected_segment_numbers, storyboard_prompts_from_rows,
    validate_first_frame_prompts, validate_frame_prompts, validate_storyboard_prompts,
    video_prompt_segments_with_reference_ids,
};
use crate::video::prompt_json::{
    VideoFramePrompt, VideoSegmentPlan, VideoShotAnalysis, parse_stored_video_segment_plan,
    parse_stored_video_shot_analysis,
};
use crate::video::prompt_records::{
    PromptWorkflowRowsContext, ReusablePromptOutputQuery, reusable_rows_are_complete,
    video_generation_asset_fingerprint,
};
use crate::video::prompt_store::PromptWorkflowMode;
use crate::video::{
    ChapterVideoPromptOptions, ChapterVideoSourceContext, FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT,
    PromptOutputLanguage, VideoService,
};

mod video;

async fn source_spans_for_prompt_preview(
    service: &VideoService,
    chapter_id: i64,
    source: &ChapterVideoSourceContext<'_>,
) -> Result<Vec<SourceSpan>, AppError> {
    let chapter = service.get_chapter(chapter_id).await?;
    Ok(source_spans_for_content(
        chapter.id,
        chapter.chapter_number,
        source.content,
    ))
}

struct PreparedVideoPromptPreviewInputs {
    source_metadata: Value,
    source_spans: Vec<SourceSpan>,
    regenerate_existing: bool,
    style: String,
    prompt_output_language: PromptOutputLanguage,
    duration_secs: i32,
    aspect_ratio: String,
    reference_image_ids: Vec<i64>,
    reference_ids: Value,
    generation_asset_fingerprint: Value,
    project_context: Option<Value>,
    available_assets: Value,
    vars: HashMap<String, Value>,
}

struct LocalPromptPreviewInput<'a, 'source> {
    project_id: i64,
    chapter_id: i64,
    source: ChapterVideoSourceContext<'source>,
    prompt_engine: &'a dyn PromptEngine,
    variables: &'a HashMap<String, Value>,
    options: ChapterVideoPromptOptions,
}

enum BoundaryFramePromptPreviewState {
    Complete(Vec<VideoFramePrompt>),
    Next(BoundaryFramePromptPreviewNext),
}

struct BoundaryFramePromptPreviewNext {
    current_frame: Value,
    available_assets: Value,
}

struct BoundaryFramePromptPreviewStateInput<'a> {
    project_id: i64,
    chapter_id: i64,
    input: &'a PreparedVideoPromptPreviewInputs,
    segment_plan: &'a [VideoSegmentPlan],
    allowed_reference_ids: &'a HashSet<i64>,
    workflow_mode: PromptWorkflowMode,
    frame_prompt_type: &'a str,
    frame_source_prompt_type: Option<&'a str>,
}

enum StoryboardPromptPreviewState {
    Complete(Vec<VideoFramePrompt>),
    Next(StoryboardPromptPreviewNext),
}

struct StoryboardPromptPreviewNext {
    current_frame: Value,
    available_assets: Value,
}

fn allowed_reference_ids_for_preview(
    available_assets: &Value,
    reference_image_ids: &[i64],
) -> Result<HashSet<i64>, AppError> {
    available_video_image_ids(available_assets, reference_image_ids)
}

struct SegmentPlanStepPreviewInput<'a, 'source> {
    prompt_engine: &'a dyn PromptEngine,
    project_id: i64,
    source: &'a ChapterVideoSourceContext<'source>,
    input: &'a PreparedVideoPromptPreviewInputs,
    project_context: Option<&'a Value>,
    shot_analysis: &'a Value,
    current_shot: &'a Value,
    previous_output_placeholder: bool,
    title: &'a str,
}

fn segment_plan_step_from_preview_input(
    input: SegmentPlanStepPreviewInput<'_, '_>,
) -> Result<VideoPromptRequestPreviewStep, AppError> {
    let SegmentPlanStepPreviewInput {
        prompt_engine,
        project_id,
        source,
        input,
        project_context,
        shot_analysis,
        current_shot,
        previous_output_placeholder,
        title,
    } = input;
    let context = VideoPlanningPromptContext {
        prompt_engine,
        project_id,
        chapter_summary: source.summary,
        chapter_analysis: source.analysis,
        project_context,
        style: &input.style,
        duration_secs: input.duration_secs,
        aspect_ratio: &input.aspect_ratio,
    };
    let user = build_segment_plan_for_shot_user(
        context,
        shot_analysis,
        current_shot,
        &input.source_spans,
    )?;
    let context_manifest = build_video_context_manifest(VideoContextManifestInput {
        prompt_engine,
        project_id,
        variables: &input.vars,
        source,
        source_spans: &input.source_spans,
        prompt_names: &["chapter_video_shot_analysis", "chapter_video_segment_plan"],
        previous_outputs: &[VideoContextPreviousOutput {
            pack_kind: VIDEO_PREVIOUS_OUTPUT_SHOT_ANALYSIS,
            prompt_name: "chapter_video_shot_analysis",
            parsed_output: shot_analysis,
            raw_output: None,
            placeholder: previous_output_placeholder,
        }],
        project_context,
        asset_context: None,
        dynamic_user_content: user.clone(),
    })?;
    let request = build_video_prompt_request(
        prompt_engine,
        "chapter_video_segment_plan",
        project_id,
        &input.vars,
        context_manifest,
        0.35,
    )?;
    step_from_request("segment_plan", title, "chapter_video_segment_plan", request)
}

fn parse_segment_plan_for_preview(
    segment_out: &str,
    input: &PreparedVideoPromptPreviewInputs,
) -> Result<(Value, Vec<VideoSegmentPlan>), AppError> {
    let segment_plan = parse_stored_video_segment_plan(segment_out, &input.source_spans)?;
    Ok((json!({ "segments": &segment_plan }), segment_plan))
}

fn frame_prompts_from_preview_context(
    frame_json: &Value,
) -> Result<Vec<VideoFramePrompt>, AppError> {
    let frames = frame_json
        .get("frames")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument("视频提示词预览缺少 frames 上下文".to_string()))?;
    frames
        .iter()
        .cloned()
        .map(|frame| {
            serde_json::from_value::<VideoFramePrompt>(frame).map_err(|err| {
                AppError::InvalidArgument(format!("视频提示词预览 frames 上下文无效: {err}"))
            })
        })
        .collect()
}

fn shot_analysis_step_from_preview_input(
    prompt_engine: &dyn PromptEngine,
    project_id: i64,
    source: &ChapterVideoSourceContext<'_>,
    input: &PreparedVideoPromptPreviewInputs,
    project_context: Option<&Value>,
    extracted_beats: &Value,
    previous_output_placeholder: bool,
) -> Result<VideoPromptRequestPreviewStep, AppError> {
    let context = VideoPlanningPromptContext {
        prompt_engine,
        project_id,
        chapter_summary: source.summary,
        chapter_analysis: source.analysis,
        project_context,
        style: &input.style,
        duration_secs: input.duration_secs,
        aspect_ratio: &input.aspect_ratio,
    };
    let user = build_shot_grouping_user(context, extracted_beats)?;
    let beat_context_output = json!({ "parsed_beats": extracted_beats });
    let context_manifest = build_video_context_manifest(VideoContextManifestInput {
        prompt_engine,
        project_id,
        variables: &input.vars,
        source,
        source_spans: &input.source_spans,
        prompt_names: &[
            "chapter_video_beat_extraction",
            "chapter_video_shot_analysis",
        ],
        previous_outputs: &[VideoContextPreviousOutput {
            pack_kind: VIDEO_PREVIOUS_OUTPUT_BEAT_EXTRACTION,
            prompt_name: "chapter_video_beat_extraction",
            parsed_output: &beat_context_output,
            raw_output: None,
            placeholder: previous_output_placeholder,
        }],
        project_context,
        asset_context: None,
        dynamic_user_content: user.clone(),
    })?;
    let request = build_video_prompt_request(
        prompt_engine,
        "chapter_video_shot_analysis",
        project_id,
        &input.vars,
        context_manifest,
        0.35,
    )?;
    step_from_request(
        "shot_analysis",
        "镜头语言分析",
        "chapter_video_shot_analysis",
        request,
    )
}

fn beat_extraction_step_from_preview_input(
    prompt_engine: &dyn PromptEngine,
    project_id: i64,
    source: &ChapterVideoSourceContext<'_>,
    input: &PreparedVideoPromptPreviewInputs,
    project_context: Option<&Value>,
) -> Result<VideoPromptRequestPreviewStep, AppError> {
    let windows = source_span_windows(&input.source_spans);
    let source_spans = windows
        .first()
        .copied()
        .unwrap_or(input.source_spans.as_slice());
    let context = VideoPlanningPromptContext {
        prompt_engine,
        project_id,
        chapter_summary: source.summary,
        chapter_analysis: source.analysis,
        project_context,
        style: &input.style,
        duration_secs: input.duration_secs,
        aspect_ratio: &input.aspect_ratio,
    };
    let user = build_beat_extraction_user(context, source_spans, 1, windows.len().max(1))?;
    let context_manifest = build_video_context_manifest(VideoContextManifestInput {
        prompt_engine,
        project_id,
        variables: &input.vars,
        source,
        source_spans: &input.source_spans,
        prompt_names: &["chapter_video_beat_extraction"],
        previous_outputs: &[],
        project_context,
        asset_context: None,
        dynamic_user_content: user.clone(),
    })?;
    let request = build_video_prompt_request(
        prompt_engine,
        "chapter_video_beat_extraction",
        project_id,
        &input.vars,
        context_manifest,
        0.25,
    )?;
    step_from_request(
        "beat_extraction",
        "动作节拍抽取",
        "chapter_video_beat_extraction",
        request,
    )
}

#[derive(Debug, Clone, Copy)]
struct SharedShotAnalysisPreviewSpec {
    workflow_mode: &'static str,
    prompt_type: &'static str,
    title: &'static str,
}

const SHARED_SHOT_ANALYSIS_PREVIEW_SPECS: &[SharedShotAnalysisPreviewSpec] =
    &[SharedShotAnalysisPreviewSpec {
        workflow_mode: "boundary_frames",
        prompt_type: "frame_shot_analysis",
        title: "视频镜头语言分析",
    }];

fn shared_shot_analysis_preview_specs(
    current_workflow_mode: &str,
) -> Vec<SharedShotAnalysisPreviewSpec> {
    let mut specs = SHARED_SHOT_ANALYSIS_PREVIEW_SPECS.to_vec();
    specs.sort_by_key(|spec| {
        if spec.workflow_mode == current_workflow_mode {
            0
        } else {
            1
        }
    });
    specs
}

fn push_placeholder_segment_plan_steps(
    steps: &mut Vec<VideoPromptRequestPreviewStep>,
    prompt_engine: &dyn PromptEngine,
    project_id: i64,
    source: &ChapterVideoSourceContext<'_>,
    input: &PreparedVideoPromptPreviewInputs,
    project_context: Option<&Value>,
    title: &str,
) -> Result<(), AppError> {
    steps.push(beat_extraction_step_from_preview_input(
        prompt_engine,
        project_id,
        source,
        input,
        project_context,
    )?);
    let placeholder_beats = placeholder_extracted_beats(input.duration_secs);
    steps.push(shot_analysis_step_from_preview_input(
        prompt_engine,
        project_id,
        source,
        input,
        project_context,
        &placeholder_beats,
        true,
    )?);
    let placeholder_shot_analysis = placeholder_shot_analysis(input.duration_secs);
    let current_shot = first_cinematic_shot_for_preview(&placeholder_shot_analysis)?;
    steps.push(segment_plan_step_from_preview_input(
        SegmentPlanStepPreviewInput {
            prompt_engine,
            project_id,
            source,
            input,
            project_context,
            shot_analysis: &placeholder_shot_analysis,
            current_shot: &current_shot,
            previous_output_placeholder: true,
            title,
        },
    )?);
    Ok(())
}

#[derive(Clone, Copy)]
enum LocalVideoPromptPreviewMode {
    BoundaryFrames,
    FirstFrameChain,
    ChatVideoBoundaryFrames,
    Storyboard,
}

#[derive(Clone, Copy)]
struct LocalVideoPromptPreviewSpec {
    mode: LocalVideoPromptPreviewMode,
    reuse_sequence: VideoPromptReuseSequence,
    segment_prompt_type: &'static str,
    segment_title: &'static str,
    segment_reused_note: &'static str,
    frame_key: &'static str,
    frame_title: &'static str,
    frame_prompt_type: &'static str,
    frame_source_prompt_type: Option<&'static str>,
    frame_prompt_name: &'static str,
    video_title: &'static str,
    video_prompt_name: &'static str,
    video_placeholder_note: &'static str,
    review_placeholder_note: &'static str,
}

impl LocalVideoPromptPreviewSpec {
    fn prompt_workflow_mode(self) -> PromptWorkflowMode {
        match self.mode {
            LocalVideoPromptPreviewMode::BoundaryFrames => PromptWorkflowMode::BoundaryFrames,
            LocalVideoPromptPreviewMode::FirstFrameChain => PromptWorkflowMode::FirstFrameChain,
            LocalVideoPromptPreviewMode::ChatVideoBoundaryFrames => PromptWorkflowMode::ChatVideo,
            LocalVideoPromptPreviewMode::Storyboard => PromptWorkflowMode::Storyboard,
        }
    }

    fn workflow_mode(self) -> &'static str {
        self.prompt_workflow_mode().as_str()
    }

    fn boundary_frames() -> Self {
        Self {
            mode: LocalVideoPromptPreviewMode::BoundaryFrames,
            reuse_sequence: VideoPromptReuseSequence::NearestPrevious,
            segment_prompt_type: "frame_segment_plan",
            segment_title: "视频片段规划",
            segment_reused_note: "已存在当前章节片段规划，视频提示词请求会直接使用该前置结果",
            frame_key: "frame_prompts",
            frame_title: "边界帧提示词",
            frame_prompt_type: "frame_prompt",
            frame_source_prompt_type: None,
            frame_prompt_name: "chapter_video_frame_prompts",
            video_title: "视频提示词",
            video_prompt_name: "chapter_video_prompt",
            video_placeholder_note: "结构预览：真实生成时 previous_steps.chapter_video_segment_plan 和 chapter_video_frame_prompts 会替换为前序模型输出；这里用占位前序结果展示单段视频提示词请求格式。",
            review_placeholder_note: "结构预览：真实生成时 draft_chapter_video_prompt 会替换为上一条视频提示词初稿模型输出；这里用占位初稿展示自检修订请求格式。",
        }
    }

    fn first_frame_chain() -> Self {
        Self {
            mode: LocalVideoPromptPreviewMode::FirstFrameChain,
            reuse_sequence: VideoPromptReuseSequence::ContiguousPrefix,
            segment_prompt_type: "frame_segment_plan",
            segment_title: "视频片段规划",
            segment_reused_note: "已存在当前章节共享片段规划，视频提示词请求会直接使用该前置结果",
            frame_key: "first_frame_prompts",
            frame_title: "初始首帧提示词",
            frame_prompt_type: "first_frame_prompt",
            frame_source_prompt_type: Some("frame_segment_plan"),
            frame_prompt_name: "chapter_video_first_frame_prompts",
            video_title: "首帧串联视频提示词",
            video_prompt_name: "chapter_video_first_frame_video_prompt",
            video_placeholder_note: "结构预览：真实生成时 previous_steps.chapter_video_segment_plan 和 chapter_video_first_frame_prompts 会替换为前序模型输出；这里用占位前序结果展示首帧串联视频提示词请求格式。",
            review_placeholder_note: "结构预览：真实生成时 draft_chapter_video_first_frame_video_prompt 会替换为上一条首帧串联视频提示词初稿模型输出；这里用占位初稿展示自检修订请求格式。",
        }
    }

    fn chat_video_boundary_frames() -> Self {
        Self {
            mode: LocalVideoPromptPreviewMode::ChatVideoBoundaryFrames,
            reuse_sequence: VideoPromptReuseSequence::NearestPrevious,
            segment_prompt_type: "frame_segment_plan",
            segment_title: "视频片段规划",
            segment_reused_note: "已存在当前章节共享片段规划，Chat 视频提示词请求会直接使用该前置结果",
            frame_key: "chat_video_frame_prompts",
            frame_title: "Chat 首尾帧提示词",
            frame_prompt_type: "chat_video_frame_prompt",
            frame_source_prompt_type: Some("frame_segment_plan"),
            frame_prompt_name: "chapter_video_frame_prompts",
            video_title: "Chat 视频提示词",
            video_prompt_name: "chapter_video_chat_prompt",
            video_placeholder_note: "结构预览：真实生成时 previous_steps.chapter_video_segment_plan 和 chapter_video_frame_prompts 会替换为 Chat 前序模型输出；这里用占位前序结果展示单段 Chat 视频提示词请求格式。",
            review_placeholder_note: "结构预览：真实生成时 draft_chapter_video_chat_prompt 会替换为上一条 Chat 视频提示词初稿模型输出；这里用占位初稿展示自检修订请求格式。",
        }
    }

    fn storyboard() -> Self {
        Self {
            mode: LocalVideoPromptPreviewMode::Storyboard,
            reuse_sequence: VideoPromptReuseSequence::ContiguousPrefix,
            segment_prompt_type: "frame_segment_plan",
            segment_title: "视频片段规划",
            segment_reused_note: "已存在当前章节共享片段规划，分镜视频提示词请求会直接使用该前置结果",
            frame_key: "storyboard_prompts",
            frame_title: "分镜提示词",
            frame_prompt_type: "storyboard_prompt",
            frame_source_prompt_type: Some("frame_segment_plan"),
            frame_prompt_name: "chapter_video_storyboard_prompts",
            video_title: "分镜视频提示词",
            video_prompt_name: "chapter_video_storyboard_video_prompt",
            video_placeholder_note: "结构预览：真实生成时 previous_steps.chapter_video_segment_plan 和 chapter_video_storyboard_prompts 会替换为前序模型输出；这里用占位前序结果展示单段分镜视频提示词请求格式。",
            review_placeholder_note: "结构预览：真实生成时 draft_chapter_video_storyboard_video_prompt 会替换为上一条分镜视频提示词初稿模型输出；这里用占位初稿展示自检修订请求格式。",
        }
    }
}

#[derive(Clone, Copy)]
enum VideoPromptReuseSequence {
    NearestPrevious,
    ContiguousPrefix,
}

fn video_prompt_target<'a, T>(
    segment_plan: &'a [VideoSegmentPlan],
    reusable_video_prompts: &'a BTreeMap<i32, T>,
    regenerate_existing: bool,
    segment_is_placeholder: bool,
    sequence: VideoPromptReuseSequence,
) -> (Option<&'a VideoSegmentPlan>, bool, Option<&'a T>) {
    let all_segments_ready = !regenerate_existing
        && !segment_is_placeholder
        && segment_plan
            .iter()
            .all(|segment| reusable_video_prompts.contains_key(&segment.segment_number));
    match sequence {
        VideoPromptReuseSequence::NearestPrevious => {
            let current_segment = segment_plan
                .iter()
                .find(|segment| !reusable_video_prompts.contains_key(&segment.segment_number))
                .or_else(|| segment_plan.first());
            let previous_shot = current_segment.and_then(|segment| {
                reusable_video_prompts
                    .range(..segment.segment_number)
                    .next_back()
                    .map(|(_, prompt)| prompt)
            });
            (current_segment, all_segments_ready, previous_shot)
        }
        VideoPromptReuseSequence::ContiguousPrefix => {
            let mut previous_completed_shot = None;
            let mut current_segment = None;
            for segment in segment_plan {
                if let Some(reusable) = reusable_video_prompts.get(&segment.segment_number) {
                    previous_completed_shot = Some(reusable);
                } else {
                    current_segment = Some(segment);
                    break;
                }
            }
            (
                current_segment.or_else(|| segment_plan.first()),
                all_segments_ready,
                previous_completed_shot,
            )
        }
    }
}

fn prompt_values_by_shot<T: Serialize>(
    prompts: BTreeMap<i32, T>,
) -> Result<BTreeMap<i32, Value>, AppError> {
    prompts
        .into_iter()
        .map(|(shot_number, prompt)| Ok((shot_number, serde_json::to_value(prompt)?)))
        .collect()
}

fn build_first_frame_prompt_preview_user(
    prompt_engine: &dyn PromptEngine,
    project_id: i64,
    source: &ChapterVideoSourceContext<'_>,
    input: &PreparedVideoPromptPreviewInputs,
    segment_json: &Value,
) -> Result<String, AppError> {
    let available_assets = frame_prompt_available_assets_context(&input.available_assets)?;
    let context = FramePromptContext {
        prompt_engine,
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
    build_first_frame_prompt_user(context, source.content)
}

fn build_boundary_frame_prompt_preview_user(
    prompt_engine: &dyn PromptEngine,
    project_id: i64,
    source: &ChapterVideoSourceContext<'_>,
    input: &PreparedVideoPromptPreviewInputs,
    segment_json: &Value,
    current_frame: &Value,
    available_assets: &Value,
) -> Result<String, AppError> {
    let context = FramePromptContext {
        prompt_engine,
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
    build_single_frame_prompt_user(context, current_frame)
}

fn build_storyboard_prompt_preview_user(
    prompt_engine: &dyn PromptEngine,
    project_id: i64,
    source: &ChapterVideoSourceContext<'_>,
    input: &PreparedVideoPromptPreviewInputs,
    segment_json: &Value,
    current_frame: &Value,
    available_assets: &Value,
) -> Result<String, AppError> {
    let context = FramePromptContext {
        prompt_engine,
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
    build_storyboard_prompt_user(context, current_frame)
}

struct FramePromptPreviewRequestInput<'a, 'source> {
    prompt_engine: &'a dyn PromptEngine,
    project_id: i64,
    source: &'a ChapterVideoSourceContext<'source>,
    input: &'a PreparedVideoPromptPreviewInputs,
    prompt_name: &'a str,
    user: String,
    segment_json: &'a Value,
    asset_context: &'a Value,
    segment_placeholder: bool,
}

fn build_frame_prompt_preview_request(
    input: FramePromptPreviewRequestInput<'_, '_>,
) -> Result<ChatRequest, AppError> {
    let FramePromptPreviewRequestInput {
        prompt_engine,
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
        prompt_engine,
        project_id,
        variables: &input.vars,
        source,
        source_spans: &input.source_spans,
        prompt_names: &["chapter_video_segment_plan", prompt_name],
        previous_outputs: &[VideoContextPreviousOutput {
            pack_kind: VIDEO_PREVIOUS_OUTPUT_SEGMENT_PLAN,
            prompt_name: "chapter_video_segment_plan",
            parsed_output: segment_json,
            raw_output: None,
            placeholder: segment_placeholder,
        }],
        project_context: input.project_context.as_ref(),
        asset_context: Some(asset_context),
        dynamic_user_content: user.clone(),
    })?;
    build_video_prompt_request(
        prompt_engine,
        prompt_name,
        project_id,
        &input.vars,
        context_manifest,
        0.45,
    )
}

struct VideoPromptPreviewRequestInput<'a, 'source> {
    prompt_engine: &'a dyn PromptEngine,
    project_id: i64,
    source: &'a ChapterVideoSourceContext<'source>,
    input: &'a PreparedVideoPromptPreviewInputs,
    prompt_name: &'a str,
    frame_prompt_name: &'a str,
    user: String,
}

fn build_video_prompt_preview_request(
    input: VideoPromptPreviewRequestInput<'_, '_>,
) -> Result<ChatRequest, AppError> {
    let VideoPromptPreviewRequestInput {
        prompt_engine,
        project_id,
        source,
        input,
        prompt_name,
        frame_prompt_name,
        user,
    } = input;
    let context_manifest = build_final_video_context_manifest(
        prompt_engine,
        project_id,
        &input.vars,
        source,
        &["chapter_video_segment_plan", frame_prompt_name, prompt_name],
        user.clone(),
    )?;
    build_video_prompt_request(
        prompt_engine,
        prompt_name,
        project_id,
        &input.vars,
        context_manifest,
        0.45,
    )
}

struct SegmentPlanPreviewStepsInput<'a, 'source> {
    prompt_engine: &'a dyn PromptEngine,
    project_id: i64,
    chapter_id: i64,
    source: &'a ChapterVideoSourceContext<'source>,
    input: &'a PreparedVideoPromptPreviewInputs,
    project_context: Option<&'a Value>,
    current_workflow_mode: &'a str,
    segment_title: &'a str,
}

fn push_video_prompt_request_step(
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

impl VideoService {
    async fn prepare_video_prompt_preview_inputs(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: &ChapterVideoSourceContext<'_>,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
        include_project_context: bool,
    ) -> Result<PreparedVideoPromptPreviewInputs, AppError> {
        let options = validate_prompt_options(options)?;
        let source_metadata = video_source_metadata(source);
        self.ensure_prompt_generation_inputs(project_id, chapter_id, &options)
            .await?;
        let source_spans = source_spans_for_prompt_preview(self, chapter_id, source).await?;
        let ChapterVideoPromptOptions {
            style,
            prompt_output_language,
            duration_secs,
            aspect_ratio,
            reference_image_ids,
            video_library_image_ids,
            regenerate_existing,
            ..
        } = options;
        let prompt_output_language =
            PromptOutputLanguage::from_optional_code(prompt_output_language.as_deref())?;
        let reference_ids = json!(reference_image_ids.clone());
        let project_context = if include_project_context {
            self.chapter_video_project_context(project_id, source.analysis)
                .await?
        } else {
            None
        };
        let available_assets = self
            .video_library_image_context(
                project_id,
                chapter_id,
                source,
                &reference_image_ids,
                &video_library_image_ids,
            )
            .await?;
        let generation_asset_fingerprint = video_generation_asset_fingerprint(
            &reference_image_ids,
            &video_library_image_ids,
            &available_assets,
        );
        let mut vars = variables.clone();
        vars.insert("style".into(), json!(style));
        vars.insert(
            "prompt_output_language".into(),
            json!(prompt_output_language.as_str()),
        );
        vars.insert(
            "prompt_output_language_instruction".into(),
            json!(prompt_output_language.instruction()),
        );
        vars.insert("duration_secs".into(), json!(duration_secs));
        vars.insert("aspect_ratio".into(), json!(aspect_ratio));
        vars.insert("reference_image_ids".into(), reference_ids.clone());
        vars.insert("available_video_assets".into(), available_assets.clone());
        Ok(PreparedVideoPromptPreviewInputs {
            source_metadata,
            source_spans,
            regenerate_existing,
            style,
            prompt_output_language,
            duration_secs,
            aspect_ratio,
            reference_image_ids,
            reference_ids,
            generation_asset_fingerprint,
            project_context,
            available_assets,
            vars,
        })
    }

    async fn reusable_first_frame_prompts_for_preview(
        &self,
        project_id: i64,
        chapter_id: i64,
        input: &PreparedVideoPromptPreviewInputs,
        segment_plan: &[VideoSegmentPlan],
        allowed_reference_ids: &HashSet<i64>,
        block_when_regenerating: bool,
    ) -> Result<Option<Vec<VideoFramePrompt>>, AppError> {
        let frame_rows = self
            .reusable_prompt_rows(
                PromptWorkflowRowsContext {
                    project_id,
                    chapter_id,
                    workflow_mode: PromptWorkflowMode::FirstFrameChain.as_str(),
                    prompt_type: "first_frame_prompt",
                    source_metadata: &input.source_metadata,
                    style: &input.style,
                    duration_secs: input.duration_secs,
                    aspect_ratio: &input.aspect_ratio,
                    reference_ids: &input.reference_ids,
                    generation_asset_fingerprint: Some(&input.generation_asset_fingerprint),
                    source_prompt_type: Some("frame_segment_plan"),
                },
                input.prompt_output_language,
            )
            .await?;
        let frame_rows = initial_first_frame_rows(frame_rows);
        if (block_when_regenerating && input.regenerate_existing)
            || !reusable_rows_are_complete(&frame_rows, FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT)
        {
            return Ok(None);
        }
        let first_frame_prompts = first_frame_prompts_from_rows(&frame_rows)?;
        validate_first_frame_prompts(&first_frame_prompts, segment_plan, allowed_reference_ids)?;
        Ok(Some(first_frame_prompts))
    }

    async fn reusable_shot_analysis_for_preview(
        &self,
        project_id: i64,
        chapter_id: i64,
        input: &PreparedVideoPromptPreviewInputs,
        current_workflow_mode: &str,
    ) -> Result<Option<(Value, VideoShotAnalysis, &'static str)>, AppError> {
        for spec in shared_shot_analysis_preview_specs(current_workflow_mode) {
            if let Some(output) = self
                .reusable_shared_shot_analysis_output(
                    project_id,
                    chapter_id,
                    spec.workflow_mode,
                    spec.prompt_type,
                    &input.source_metadata,
                )
                .await?
            {
                let analysis = parse_stored_video_shot_analysis(
                    &output,
                    &input.source_spans,
                    input.duration_secs,
                )?;
                let analysis_json = serde_json::to_value(&analysis).map_err(|err| {
                    AppError::InvalidArgument(format!(
                        "chapter_video_shot_analysis 派生结果序列化失败: {err}"
                    ))
                })?;
                return Ok(Some((analysis_json, analysis, spec.title)));
            }
        }
        Ok(None)
    }

    async fn push_segment_plan_preview_steps(
        &self,
        steps: &mut Vec<VideoPromptRequestPreviewStep>,
        input: SegmentPlanPreviewStepsInput<'_, '_>,
    ) -> Result<(), AppError> {
        let SegmentPlanPreviewStepsInput {
            prompt_engine,
            project_id,
            chapter_id,
            source,
            input,
            project_context,
            current_workflow_mode,
            segment_title,
        } = input;
        if let Some((shot_analysis_json, _shot_analysis, title)) = self
            .reusable_shot_analysis_for_preview(
                project_id,
                chapter_id,
                input,
                current_workflow_mode,
            )
            .await?
        {
            steps.push(reused_step(
                "shot_analysis",
                "镜头语言分析",
                "chapter_video_shot_analysis",
                &format!("已存在可复用{title}，本次生成会跳过该 LLM 请求"),
            ));
            steps.push(segment_plan_step_from_preview_input(
                SegmentPlanStepPreviewInput {
                    prompt_engine,
                    project_id,
                    source,
                    input,
                    project_context,
                    shot_analysis: &shot_analysis_json,
                    current_shot: &first_cinematic_shot_for_preview(&shot_analysis_json)?,
                    previous_output_placeholder: false,
                    title: segment_title,
                },
            )?);
            return Ok(());
        }
        push_placeholder_segment_plan_steps(
            steps,
            prompt_engine,
            project_id,
            source,
            input,
            project_context,
            segment_title,
        )
    }

    async fn boundary_frame_prompt_preview_state(
        &self,
        input: BoundaryFramePromptPreviewStateInput<'_>,
    ) -> Result<BoundaryFramePromptPreviewState, AppError> {
        let BoundaryFramePromptPreviewStateInput {
            project_id,
            chapter_id,
            input,
            segment_plan,
            allowed_reference_ids,
            workflow_mode,
            frame_prompt_type,
            frame_source_prompt_type,
        } = input;
        if segment_plan.is_empty() {
            return Err(AppError::InvalidArgument(
                "视频片段规划不能为空".to_string(),
            ));
        }

        let frame_rows = if input.regenerate_existing {
            Vec::new()
        } else {
            self.reusable_prompt_rows(
                PromptWorkflowRowsContext {
                    project_id,
                    chapter_id,
                    workflow_mode: workflow_mode.as_str(),
                    prompt_type: frame_prompt_type,
                    source_metadata: &input.source_metadata,
                    style: &input.style,
                    duration_secs: input.duration_secs,
                    aspect_ratio: &input.aspect_ratio,
                    reference_ids: &input.reference_ids,
                    generation_asset_fingerprint: Some(&input.generation_asset_fingerprint),
                    source_prompt_type: frame_source_prompt_type,
                },
                input.prompt_output_language,
            )
            .await?
        };
        let frame_image_lookup_rows = if input.regenerate_existing {
            Vec::new()
        } else {
            boundary_frame_rows_for_image_lookup(FrameImageLookupRowsQuery {
                service: self,
                project_id,
                chapter_id,
                context: FrameImageLookupContext {
                    workflow_mode,
                    prompt_type: frame_prompt_type,
                    source_metadata: &input.source_metadata,
                    style: &input.style,
                    aspect_ratio: &input.aspect_ratio,
                    source_prompt_type: frame_source_prompt_type,
                    prompt_output_language: input.prompt_output_language,
                },
            })
            .await?
        };
        let frame_image_id_by_number = boundary_frame_image_ids_by_number(
            self,
            project_id,
            chapter_id,
            &frame_image_lookup_rows,
        )
        .await?;
        let mut allowed_boundary_reference_ids = allowed_reference_ids.clone();
        allowed_boundary_reference_ids.extend(frame_image_id_by_number.values().copied());
        let prefix_rows = boundary_frame_prompt_rows_prefix(
            frame_rows,
            segment_plan,
            &allowed_boundary_reference_ids,
            &frame_image_id_by_number,
        )?;
        let mut completed_frames = frame_prompts_from_rows(&prefix_rows)?;
        let reusable_prefix_len = reusable_frame_prefix_len_with_previous_images(
            &completed_frames,
            &frame_image_id_by_number,
        );
        completed_frames.truncate(reusable_prefix_len);

        let expected_frame_count = segment_plan.len() * 2;
        if completed_frames.len() == expected_frame_count {
            validate_frame_prompts(
                &completed_frames,
                segment_plan,
                &allowed_boundary_reference_ids,
            )?;
            return Ok(BoundaryFramePromptPreviewState::Complete(completed_frames));
        }

        Ok(BoundaryFramePromptPreviewState::Next(
            Self::boundary_frame_prompt_preview_next(
                input,
                segment_plan,
                completed_frames,
                &frame_image_id_by_number,
            )?,
        ))
    }

    fn placeholder_boundary_frame_prompt_preview_state(
        input: &PreparedVideoPromptPreviewInputs,
        segment_plan: &[VideoSegmentPlan],
    ) -> Result<BoundaryFramePromptPreviewNext, AppError> {
        Self::boundary_frame_prompt_preview_next(input, segment_plan, Vec::new(), &HashMap::new())
    }

    fn boundary_frame_prompt_preview_next(
        input: &PreparedVideoPromptPreviewInputs,
        segment_plan: &[VideoSegmentPlan],
        completed_frames: Vec<VideoFramePrompt>,
        frame_image_id_by_number: &HashMap<i32, i64>,
    ) -> Result<BoundaryFramePromptPreviewNext, AppError> {
        if segment_plan.is_empty() {
            return Err(AppError::InvalidArgument(
                "视频片段规划不能为空".to_string(),
            ));
        }
        let frame_number = (completed_frames.len() + 1) as i32;
        let previous_frame_image_id = if frame_number > 1 {
            frame_image_id_by_number.get(&(frame_number - 1)).copied()
        } else {
            None
        };
        let available_assets = available_assets_with_previous_boundary_frame(
            &input.available_assets,
            previous_frame_image_id,
            completed_frames.last(),
        );
        let prompt_available_assets = frame_prompt_available_assets_context(&available_assets)?;
        let current_frame = boundary_frame_generation_context(
            segment_plan,
            &completed_frames,
            frame_number,
            previous_boundary_frame_asset_context(&available_assets, previous_frame_image_id),
        )?;

        Ok(BoundaryFramePromptPreviewNext {
            current_frame,
            available_assets: prompt_available_assets,
        })
    }

    async fn storyboard_prompt_preview_state(
        &self,
        project_id: i64,
        chapter_id: i64,
        input: &PreparedVideoPromptPreviewInputs,
        segment_plan: &[VideoSegmentPlan],
        allowed_reference_ids: &HashSet<i64>,
    ) -> Result<StoryboardPromptPreviewState, AppError> {
        if segment_plan.is_empty() {
            return Err(AppError::InvalidArgument(
                "视频片段规划不能为空".to_string(),
            ));
        }

        let reusable_rows = if input.regenerate_existing {
            Vec::new()
        } else {
            self.reusable_prompt_rows(
                PromptWorkflowRowsContext {
                    project_id,
                    chapter_id,
                    workflow_mode: PromptWorkflowMode::Storyboard.as_str(),
                    prompt_type: "storyboard_prompt",
                    source_metadata: &input.source_metadata,
                    style: &input.style,
                    duration_secs: input.duration_secs,
                    aspect_ratio: &input.aspect_ratio,
                    reference_ids: &input.reference_ids,
                    generation_asset_fingerprint: Some(&input.generation_asset_fingerprint),
                    source_prompt_type: Some("frame_segment_plan"),
                },
                input.prompt_output_language,
            )
            .await?
        };

        let expected_count = expected_video_unit_count(segment_plan);
        let mut prefix_rows = Vec::new();
        for row in reusable_rows {
            if prefix_rows.len() >= expected_count {
                break;
            }
            let expected_number = (prefix_rows.len() + 1) as i32;
            if row.shot_number == expected_number {
                if storyboard_prompts_from_rows(std::slice::from_ref(&row)).is_err() {
                    break;
                }
                prefix_rows.push(row);
            } else if row.shot_number > expected_number {
                break;
            }
        }

        let storyboard_prompts = storyboard_prompts_from_rows(&prefix_rows)?;
        if storyboard_prompts.len() == expected_count {
            validate_storyboard_prompts(&storyboard_prompts, segment_plan, allowed_reference_ids)?;
            return Ok(StoryboardPromptPreviewState::Complete(storyboard_prompts));
        }

        Ok(StoryboardPromptPreviewState::Next(
            Self::storyboard_prompt_preview_next(input, segment_plan, storyboard_prompts.len())?,
        ))
    }

    fn placeholder_storyboard_prompt_preview_state(
        input: &PreparedVideoPromptPreviewInputs,
        segment_plan: &[VideoSegmentPlan],
    ) -> Result<StoryboardPromptPreviewNext, AppError> {
        Self::storyboard_prompt_preview_next(input, segment_plan, 0)
    }

    fn storyboard_prompt_preview_next(
        input: &PreparedVideoPromptPreviewInputs,
        segment_plan: &[VideoSegmentPlan],
        completed_count: usize,
    ) -> Result<StoryboardPromptPreviewNext, AppError> {
        let expected_frame_count = expected_video_unit_count(segment_plan);
        if completed_count >= expected_frame_count {
            return Err(AppError::InvalidArgument(format!(
                "分镜提示词预览已完成 {completed_count}/{expected_frame_count} 个视频单元，没有待生成分镜"
            )));
        }
        let unit_number = (completed_count + 1) as i32;
        let expected_segment_numbers =
            storyboard_expected_segment_numbers(segment_plan, unit_number)?;
        let segment_number = expected_segment_numbers.first().copied().ok_or_else(|| {
            AppError::InvalidArgument("分镜提示词预览缺少待生成视频单元源片段".to_string())
        })?;
        let segment = segment_plan
            .iter()
            .find(|segment| segment.segment_number == segment_number)
            .ok_or_else(|| {
                AppError::InvalidArgument("分镜提示词预览缺少待生成视频单元".to_string())
            })?;
        let next_segment = expected_segment_numbers.get(1).and_then(|segment_number| {
            segment_plan
                .iter()
                .find(|segment| segment.segment_number == *segment_number)
        });
        let current_frame = json!({
            "frame_number": unit_number,
            "total_frame_count": expected_frame_count,
            "role": if next_segment.is_some() { "transition_storyboard" } else { "content_storyboard" },
            "video_unit": {
                "unit_number": unit_number,
                "unit_type": if next_segment.is_some() { "transition" } else { "content" },
                "source_segment_number": if next_segment.is_some() { Value::Null } else { Value::from(segment.segment_number) },
                "from_segment_number": segment.segment_number,
                "to_segment_number": next_segment.map(|segment| segment.segment_number),
            },
            "expected_segment_numbers": expected_segment_numbers,
            "required_duration_secs": segment.duration_secs,
            "required_aspect_ratio": &segment.aspect_ratio,
            "previous_segment": completed_count.checked_sub(1).and_then(|idx| segment_plan.get(idx / 2)),
            "current_segment": if next_segment.is_none() { Some(segment) } else { None },
            "next_segment": next_segment,
            "reference_slot_instruction": "后端提供的 constraints.reference_slots 顺序就是本帧成图输入顺序；frame_prompt 只能按槽位顺序写参考图 1、参考图 2，不要输出 reference_image_ids，不得写 image_id、图片 ID、available_assets、候选素材标签或原始图片数字 ID。",
            "continuity_instruction": "这是当前视频片段的多格分镜板，用于表达本段核心构图、动作姿态、空间关系和光影，不是首帧或尾帧。"
        });
        Ok(StoryboardPromptPreviewNext {
            current_frame,
            available_assets: frame_prompt_available_assets_context(&input.available_assets)?,
        })
    }

    pub async fn preview_chapter_frame_prompt_requests(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: ChapterVideoSourceContext<'_>,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.preview_local_frame_prompt_requests(
            LocalPromptPreviewInput {
                project_id,
                chapter_id,
                source,
                prompt_engine,
                variables,
                options,
            },
            LocalVideoPromptPreviewSpec::boundary_frames(),
        )
        .await
    }

    pub async fn preview_chapter_chat_video_frame_prompt_requests(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: ChapterVideoSourceContext<'_>,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.preview_local_frame_prompt_requests(
            LocalPromptPreviewInput {
                project_id,
                chapter_id,
                source,
                prompt_engine,
                variables,
                options,
            },
            LocalVideoPromptPreviewSpec::chat_video_boundary_frames(),
        )
        .await
    }

    pub async fn preview_chapter_storyboard_prompt_requests(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: ChapterVideoSourceContext<'_>,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.preview_local_frame_prompt_requests(
            LocalPromptPreviewInput {
                project_id,
                chapter_id,
                source,
                prompt_engine,
                variables,
                options,
            },
            LocalVideoPromptPreviewSpec::storyboard(),
        )
        .await
    }

    async fn preview_local_frame_prompt_requests(
        &self,
        request: LocalPromptPreviewInput<'_, '_>,
        spec: LocalVideoPromptPreviewSpec,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        let LocalPromptPreviewInput {
            project_id,
            chapter_id,
            source,
            prompt_engine,
            variables,
            options,
        } = request;
        let input = self
            .prepare_video_prompt_preview_inputs(
                project_id, chapter_id, &source, variables, options, true,
            )
            .await?;

        let mut steps = Vec::new();
        let segment_out = self
            .reusable_prompt_output(ReusablePromptOutputQuery {
                project_id,
                chapter_id,
                prompt_type: spec.segment_prompt_type,
                source_metadata: &input.source_metadata,
                style: &input.style,
                duration_secs: input.duration_secs,
                aspect_ratio: &input.aspect_ratio,
                reference_ids: &input.reference_ids,
                prompt_output_language: input.prompt_output_language,
            })
            .await?;
        let frame_preview_title = match spec.mode {
            LocalVideoPromptPreviewMode::BoundaryFrames => spec.segment_title,
            LocalVideoPromptPreviewMode::FirstFrameChain
            | LocalVideoPromptPreviewMode::ChatVideoBoundaryFrames
            | LocalVideoPromptPreviewMode::Storyboard => spec.segment_title,
        };
        let preview_from_steps = |steps| match spec.mode {
            LocalVideoPromptPreviewMode::BoundaryFrames => preview_boundary_from_steps(steps),
            LocalVideoPromptPreviewMode::FirstFrameChain => preview_first_frame_from_steps(steps),
            LocalVideoPromptPreviewMode::ChatVideoBoundaryFrames => {
                preview_chat_boundary_from_steps(steps)
            }
            LocalVideoPromptPreviewMode::Storyboard => preview_storyboard_from_steps(steps),
        };
        let Some(segment_out) = segment_out else {
            self.push_segment_plan_preview_steps(
                &mut steps,
                SegmentPlanPreviewStepsInput {
                    prompt_engine,
                    project_id,
                    chapter_id,
                    source: &source,
                    input: &input,
                    project_context: input.project_context.as_ref(),
                    current_workflow_mode: spec.workflow_mode(),
                    segment_title: frame_preview_title,
                },
            )
            .await?;
            let placeholder_segment = placeholder_segment_plan();
            let placeholder_segment_plan = placeholder_segments();
            let frame_user = match spec.mode {
                LocalVideoPromptPreviewMode::BoundaryFrames
                | LocalVideoPromptPreviewMode::ChatVideoBoundaryFrames => {
                    let next = Self::placeholder_boundary_frame_prompt_preview_state(
                        &input,
                        &placeholder_segment_plan,
                    )?;
                    build_boundary_frame_prompt_preview_user(
                        prompt_engine,
                        project_id,
                        &source,
                        &input,
                        &placeholder_segment,
                        &next.current_frame,
                        &next.available_assets,
                    )?
                }
                LocalVideoPromptPreviewMode::FirstFrameChain => {
                    build_first_frame_prompt_preview_user(
                        prompt_engine,
                        project_id,
                        &source,
                        &input,
                        &placeholder_segment,
                    )?
                }
                LocalVideoPromptPreviewMode::Storyboard => {
                    let next = Self::placeholder_storyboard_prompt_preview_state(
                        &input,
                        &placeholder_segment_plan,
                    )?;
                    build_storyboard_prompt_preview_user(
                        prompt_engine,
                        project_id,
                        &source,
                        &input,
                        &placeholder_segment,
                        &next.current_frame,
                        &next.available_assets,
                    )?
                }
            };
            let frame_request =
                build_frame_prompt_preview_request(FramePromptPreviewRequestInput {
                    prompt_engine,
                    project_id,
                    source: &source,
                    input: &input,
                    prompt_name: spec.frame_prompt_name,
                    user: frame_user,
                    segment_json: &placeholder_segment,
                    asset_context: &input.available_assets,
                    segment_placeholder: true,
                })?;
            steps.push(placeholder_step_from_request(
                spec.frame_key,
                spec.frame_title,
                spec.frame_prompt_name,
                frame_request,
                "结构预览：真实生成时 draft 会替换为上一步视频片段规划模型输出；这里用占位片段展示将发送给模型的请求格式。",
            )?);
            return Ok(preview_from_steps(steps));
        };

        steps.push(reused_step(
            "segment_plan",
            frame_preview_title,
            "chapter_video_segment_plan",
            "已存在可复用片段规划，本次生成会跳过该 LLM 请求",
        ));
        let (segment_json, segment_plan) = parse_segment_plan_for_preview(&segment_out, &input)?;
        let allowed_reference_ids =
            allowed_reference_ids_for_preview(&input.available_assets, &input.reference_image_ids)?;

        match spec.mode {
            LocalVideoPromptPreviewMode::BoundaryFrames
            | LocalVideoPromptPreviewMode::ChatVideoBoundaryFrames => {
                match self
                    .boundary_frame_prompt_preview_state(BoundaryFramePromptPreviewStateInput {
                        project_id,
                        chapter_id,
                        input: &input,
                        segment_plan: &segment_plan,
                        allowed_reference_ids: &allowed_reference_ids,
                        workflow_mode: spec.prompt_workflow_mode(),
                        frame_prompt_type: spec.frame_prompt_type,
                        frame_source_prompt_type: spec.frame_source_prompt_type,
                    })
                    .await?
                {
                    BoundaryFramePromptPreviewState::Complete(_) => {
                        steps.push(reused_step(
                            spec.frame_key,
                            spec.frame_title,
                            spec.frame_prompt_name,
                            &format!(
                                "已存在完整可复用{}，本次生成会跳过该 LLM 请求",
                                spec.frame_title
                            ),
                        ));
                    }
                    BoundaryFramePromptPreviewState::Next(next) => {
                        let user = build_boundary_frame_prompt_preview_user(
                            prompt_engine,
                            project_id,
                            &source,
                            &input,
                            &segment_json,
                            &next.current_frame,
                            &next.available_assets,
                        )?;
                        let request =
                            build_frame_prompt_preview_request(FramePromptPreviewRequestInput {
                                prompt_engine,
                                project_id,
                                source: &source,
                                input: &input,
                                prompt_name: spec.frame_prompt_name,
                                user,
                                segment_json: &segment_json,
                                asset_context: &next.available_assets,
                                segment_placeholder: false,
                            })?;
                        steps.push(step_from_request(
                            spec.frame_key,
                            spec.frame_title,
                            spec.frame_prompt_name,
                            request,
                        )?);
                    }
                }
                return Ok(preview_from_steps(steps));
            }
            LocalVideoPromptPreviewMode::FirstFrameChain => {
                if self
                    .reusable_first_frame_prompts_for_preview(
                        project_id,
                        chapter_id,
                        &input,
                        &segment_plan,
                        &allowed_reference_ids,
                        true,
                    )
                    .await?
                    .is_none()
                {
                    let user = build_first_frame_prompt_preview_user(
                        prompt_engine,
                        project_id,
                        &source,
                        &input,
                        &segment_json,
                    )?;
                    let request =
                        build_frame_prompt_preview_request(FramePromptPreviewRequestInput {
                            prompt_engine,
                            project_id,
                            source: &source,
                            input: &input,
                            prompt_name: spec.frame_prompt_name,
                            user,
                            segment_json: &segment_json,
                            asset_context: &input.available_assets,
                            segment_placeholder: false,
                        })?;
                    steps.push(step_from_request(
                        spec.frame_key,
                        spec.frame_title,
                        spec.frame_prompt_name,
                        request,
                    )?);
                    return Ok(preview_from_steps(steps));
                }

                steps.push(reused_step(
                    spec.frame_key,
                    spec.frame_title,
                    spec.frame_prompt_name,
                    &format!(
                        "已存在完整可复用{}，本次生成会跳过该 LLM 请求",
                        spec.frame_title
                    ),
                ));
            }
            LocalVideoPromptPreviewMode::Storyboard => {
                match self
                    .storyboard_prompt_preview_state(
                        project_id,
                        chapter_id,
                        &input,
                        &segment_plan,
                        &allowed_reference_ids,
                    )
                    .await?
                {
                    StoryboardPromptPreviewState::Complete(_) => {
                        steps.push(reused_step(
                            spec.frame_key,
                            spec.frame_title,
                            spec.frame_prompt_name,
                            &format!(
                                "已存在完整可复用{}，本次生成会跳过该 LLM 请求",
                                spec.frame_title
                            ),
                        ));
                    }
                    StoryboardPromptPreviewState::Next(next) => {
                        let user = build_storyboard_prompt_preview_user(
                            prompt_engine,
                            project_id,
                            &source,
                            &input,
                            &segment_json,
                            &next.current_frame,
                            &next.available_assets,
                        )?;
                        let request =
                            build_frame_prompt_preview_request(FramePromptPreviewRequestInput {
                                prompt_engine,
                                project_id,
                                source: &source,
                                input: &input,
                                prompt_name: spec.frame_prompt_name,
                                user,
                                segment_json: &segment_json,
                                asset_context: &next.available_assets,
                                segment_placeholder: false,
                            })?;
                        steps.push(step_from_request(
                            spec.frame_key,
                            spec.frame_title,
                            spec.frame_prompt_name,
                            request,
                        )?);
                    }
                }
            }
        }

        Ok(preview_from_steps(steps))
    }

    pub async fn preview_chapter_first_frame_prompt_requests(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: ChapterVideoSourceContext<'_>,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.preview_local_frame_prompt_requests(
            LocalPromptPreviewInput {
                project_id,
                chapter_id,
                source,
                prompt_engine,
                variables,
                options,
            },
            LocalVideoPromptPreviewSpec::first_frame_chain(),
        )
        .await
    }
}
