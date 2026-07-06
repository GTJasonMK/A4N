use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

mod boundary_context;
mod frame_chain;
mod records;
mod reuse;
#[cfg(test)]
mod tests;
mod video_chain;

use crate::llm_context::LlmContextManifest;
use a4n_core::error::AppError;
use a4n_core::models::ai::generated_image;
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_engine::llm::provider::{ChatRequest, LlmProvider, UsageInfo};
use a4n_engine::prompt::engine::PromptEngine;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::chapter::source_spans::{SourceSpan, source_spans_for_content};
use crate::generation::{
    context_aware_chat_request_trace_json, generation_checkpoint_fingerprint_sha256,
};
use crate::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    required_llm_call_workflow_run_id, save_completed_llm_call_checkpoint,
};
use crate::prompt_language::metadata_matches_prompt_output_language;
use crate::source::source_content_sha256;
use crate::workflow::{
    UpsertWorkflowCheckpointInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WorkflowRunService,
};

use super::super::library::{available_video_image_ids, ensure_video_reference_ids_known};
use super::super::prompt_context::{
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
use super::super::prompt_json::{
    ChatVideoPromptShot, VideoActionBeat, VideoCinematicShot, VideoFramePrompt,
    VideoFramePromptDefaults, VideoPromptShot, VideoSegmentPlan, VideoShotAnalysis,
    parse_chat_video_prompt_shots, parse_stored_video_segment_plan_for_reuse,
    parse_stored_video_shot_analysis, parse_video_beat_extraction_response,
    parse_video_frame_prompts_with_defaults, parse_video_prompt_shots,
    parse_video_segment_plan_input_values, parse_video_segment_plan_response,
    parse_video_shot_grouping_response, parse_video_step_object, validate_direct_prompt_text,
    validate_reference_label_text, validate_segment_plan_against_shot_analysis,
};
use super::super::prompt_records::{PromptWorkflowRowsContext, video_generation_asset_fingerprint};
use super::super::prompt_store::{
    NewChapterVideoPrompt, NewChapterVideoPromptInput, PromptWorkflowMode,
    VideoPromptGenerationTrace, new_prompt_draft,
};
use super::super::{
    BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION, ChapterVideoPromptOptions,
    ChapterVideoPromptStageInput, ChapterVideoSourceContext, FIRST_FRAME_CHAIN_CONTRACT_VERSION,
    FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT, FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
    PromptOutputLanguage, SHOT_ANALYSIS_CONTRACT_VERSION, STORYBOARD_PROMPT_CONTRACT_VERSION,
    VideoPromptChunk, VideoPromptProgress, VideoService,
};
use super::{
    FieldMilestone, ProgressSpan, RenderedVideoPromptTrace, StructuredJsonProgressSpec,
    VideoPromptLlmCallTraceContext, VideoPromptRenderInput, VideoPromptStepProgress,
    build_video_prompt_request_with_call_trace, render_and_chat_trace_with_context,
    report_video_prompt_progress,
};

use self::boundary_context::apply_boundary_frame_style_template;
#[cfg(test)]
use self::boundary_context::frame_prompt_with_style_template;
pub(in crate::video) use self::boundary_context::{
    available_assets_with_previous_boundary_frame, boundary_frame_generation_context,
    frame_prompt_available_assets_context, previous_boundary_frame_asset_context,
    reference_slots_for_frame,
};
use self::frame_chain::{
    FramePromptChainInput, SegmentPlanForSpecInput, ShotAnalysisPreparationInput,
    generate_chapter_frame_prompt_chain, generate_chapter_segment_plan_for_spec,
    prepare_shot_analysis, shot_analysis_spec_for_workflow,
};
use self::records::{
    ChatVideoPromptDraftsInput, FirstFramePromptDraftsInput, FirstFrameVideoPromptDraftsInput,
    FramePromptDraftsInput, StoryboardPromptDraftsInput, StoryboardVideoPromptDraftsInput,
    VideoPromptDraftCommonInput, VideoPromptDraftsInput, chat_video_prompt_drafts,
    complete_reusable_rows, first_frame_prompt_drafts, first_frame_video_prompt_drafts,
    frame_prompt_drafts, prompt_row_draft, storyboard_prompt_drafts,
    storyboard_video_prompt_drafts, validate_chat_video_prompts,
    validate_partial_chat_video_prompts, validate_partial_video_prompts, validate_video_prompts,
    video_prompt_drafts,
};
pub(in crate::video) use self::records::{
    first_frame_prompts_from_rows, frame_prompts_from_rows, initial_first_frame_rows,
    reusable_boundary_video_prompts_from_rows, reusable_chat_video_prompts_from_rows,
    reusable_first_frame_video_prompts_from_rows, storyboard_prompts_from_rows,
    validate_first_frame_prompts, validate_frame_prompts, validate_storyboard_prompts,
};
use self::reuse::*;
pub(in crate::video) use self::reuse::{
    FrameImageLookupContext, FrameImageLookupRowsQuery, boundary_frame_image_ids_by_number,
    boundary_frame_rows_for_image_lookup, storyboard_frame_image_ids_by_number,
    video_prompt_segments_with_reference_ids,
};

const FINAL_VIDEO_PROMPT_OUTPUT_CONTRACT_VERSION: &str =
    "final_video_prompt_dialogue_cues_no_model_source_spans_v1";

fn parse_single_video_prompt_shot(
    raw: &str,
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
    expected_shot_number: i32,
    context: &str,
) -> Result<VideoPromptShot, AppError> {
    let expected_segment = segment_by_number
        .get(&expected_shot_number)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "{context} 期望片段 #{} 不存在",
                expected_shot_number
            ))
        })?;
    let scoped_segment = HashMap::from([(expected_shot_number, *expected_segment)]);
    let mut shots = parse_video_prompt_shots(raw, &scoped_segment, prompt_output_language)?;
    if shots.len() != 1 {
        return Err(AppError::InvalidArgument(format!(
            "{context} 必须只返回当前片段 1 条 shot，实际返回 {} 条",
            shots.len()
        )));
    }
    let shot = shots.remove(0);
    if shot.shot_number != expected_shot_number {
        return Err(AppError::InvalidArgument(format!(
            "{context} 返回的 shot_number 必须为 {}，实际为 {}",
            expected_shot_number, shot.shot_number
        )));
    }
    Ok(shot)
}

fn parse_single_chat_video_prompt_shot(
    raw: &str,
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
    expected_shot_number: i32,
    context: &str,
) -> Result<ChatVideoPromptShot, AppError> {
    let expected_segment = segment_by_number
        .get(&expected_shot_number)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "{context} 期望片段 #{} 不存在",
                expected_shot_number
            ))
        })?;
    let scoped_segment = HashMap::from([(expected_shot_number, *expected_segment)]);
    let mut shots = parse_chat_video_prompt_shots(raw, &scoped_segment, prompt_output_language)?;
    if shots.len() != 1 {
        return Err(AppError::InvalidArgument(format!(
            "{context} 必须只返回当前片段 1 条 shot，实际返回 {} 条",
            shots.len()
        )));
    }
    let shot = shots.remove(0);
    if shot.shot_number != expected_shot_number {
        return Err(AppError::InvalidArgument(format!(
            "{context} 返回的 shot_number 必须为 {}，实际为 {}",
            expected_shot_number, shot.shot_number
        )));
    }
    Ok(shot)
}

fn parse_single_video_frame_prompt(
    raw: &str,
    defaults: VideoFramePromptDefaults<'_>,
    prompt_output_language: PromptOutputLanguage,
    expected_frame_number: i32,
    context: &str,
) -> Result<VideoFramePrompt, AppError> {
    let mut frames =
        parse_video_frame_prompts_with_defaults(raw, &[defaults], prompt_output_language)?;
    if frames.len() != 1 {
        return Err(AppError::InvalidArgument(format!(
            "{context} 必须只返回当前边界帧 1 条 frame，实际返回 {} 条",
            frames.len()
        )));
    }
    let frame = frames.remove(0);
    if frame.frame_number != expected_frame_number {
        return Err(AppError::InvalidArgument(format!(
            "{context} 返回的 frame_number 必须为 {}，实际为 {}",
            expected_frame_number, frame.frame_number
        )));
    }
    Ok(frame)
}

fn parse_generated_segment_plan_with_current_config(
    raw: &str,
    source_spans: &[SourceSpan],
    shot_analysis: &VideoShotAnalysis,
    duration_secs: i32,
    default_aspect_ratio: &str,
) -> Result<(Value, Vec<VideoSegmentPlan>), AppError> {
    let segment_plan = parse_video_segment_plan_response(
        raw,
        source_spans,
        shot_analysis,
        duration_secs,
        default_aspect_ratio,
    )?;
    let segment_json = json!({ "segments": &segment_plan });
    Ok((segment_json, segment_plan))
}

const SHOT_ANALYSIS_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("beat_ids", "动作节拍分组", 1),
    FieldMilestone::new("title", "标题", 1),
    FieldMilestone::new("scene_intent", "镜头意图", 1),
    FieldMilestone::new("start_state", "起始状态", 1),
    FieldMilestone::new("end_state", "结尾状态", 1),
    FieldMilestone::new("continuity_handoff", "连续性交接", 1),
];

const BEAT_EXTRACTION_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("visible_action", "可见动作", 2),
    FieldMilestone::new("natural_seconds", "自然耗时", 1),
    FieldMilestone::new("required", "必要性", 1),
    FieldMilestone::new("source_span_ids", "来源片段", 1),
];

const SEGMENT_PLAN_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("title", "标题", 1),
    FieldMilestone::new("scene_summary", "剧情状态", 1),
    FieldMilestone::new("blocking_summary", "调度摘要", 1),
    FieldMilestone::new("emotion_action_goals", "情绪动作目标", 1),
    FieldMilestone::new("video_intent", "视频意图", 1),
    FieldMilestone::new("emotional_arc", "情绪弧线", 1),
    FieldMilestone::new("material_coverage", "素材覆盖", 1),
];

const FRAME_PROMPT_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("title", "标题", 1),
    FieldMilestone::new("scene_summary", "剧情状态", 1),
    FieldMilestone::new("frame_prompt", "画面提示词", 2),
    FieldMilestone::new("blocking_state", "调度状态", 1),
    FieldMilestone::new("lighting_state", "光影状态", 1),
    FieldMilestone::new("negative_prompt", "负向提示词", 1),
];

const VIDEO_PROMPT_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("title", "标题", 1),
    FieldMilestone::new("scene_summary", "剧情状态", 1),
    FieldMilestone::new("visual_prompt", "画面提示词", 2),
    FieldMilestone::new("motion_prompt", "动作运镜提示词", 2),
    FieldMilestone::new("negative_prompt", "负向提示词", 1),
    FieldMilestone::new("dialogue_cues", "对白提示", 1),
];

const CHAT_VIDEO_PROMPT_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("title", "标题", 1),
    FieldMilestone::new("scene_summary", "剧情状态", 1),
    FieldMilestone::new("chat_prompt", "Chat 视频提示词", 3),
    FieldMilestone::new("negative_prompt", "负向提示词", 1),
];

fn shot_analysis_progress_spec(expected_items: Option<usize>) -> StructuredJsonProgressSpec {
    StructuredJsonProgressSpec {
        root_array_key: "cinematic_shots",
        item_label: "镜头分析",
        expected_items,
        milestones: SHOT_ANALYSIS_PROGRESS_FIELDS,
    }
}

fn beat_extraction_progress_spec(expected_items: Option<usize>) -> StructuredJsonProgressSpec {
    StructuredJsonProgressSpec {
        root_array_key: "beats",
        item_label: "动作节拍",
        expected_items,
        milestones: BEAT_EXTRACTION_PROGRESS_FIELDS,
    }
}

fn segment_plan_progress_spec(expected_items: Option<usize>) -> StructuredJsonProgressSpec {
    StructuredJsonProgressSpec {
        root_array_key: "segments",
        item_label: "片段规划",
        expected_items,
        milestones: SEGMENT_PLAN_PROGRESS_FIELDS,
    }
}

fn frame_prompt_progress_spec(
    item_label: &'static str,
    expected_items: Option<usize>,
) -> StructuredJsonProgressSpec {
    StructuredJsonProgressSpec {
        root_array_key: "frames",
        item_label,
        expected_items,
        milestones: FRAME_PROMPT_PROGRESS_FIELDS,
    }
}

fn video_prompt_progress_spec(is_chat_video: bool) -> StructuredJsonProgressSpec {
    if is_chat_video {
        StructuredJsonProgressSpec {
            root_array_key: "shots",
            item_label: "Chat 视频提示词",
            expected_items: Some(1),
            milestones: CHAT_VIDEO_PROMPT_PROGRESS_FIELDS,
        }
    } else {
        StructuredJsonProgressSpec {
            root_array_key: "shots",
            item_label: "视频提示词",
            expected_items: Some(1),
            milestones: VIDEO_PROMPT_PROGRESS_FIELDS,
        }
    }
}

fn attach_generation_trace_to_draft(
    drafts: Vec<NewChapterVideoPrompt>,
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
    shot_number: i32,
    trace: VideoPromptGenerationTrace,
) -> Vec<NewChapterVideoPrompt> {
    drafts
        .into_iter()
        .map(|draft| {
            if draft.matches_identity(workflow_mode, prompt_type, shot_number) {
                draft.with_generation_trace(trace.clone())
            } else {
                draft
            }
        })
        .collect()
}

fn attach_generation_trace_to_all_drafts(
    drafts: Vec<NewChapterVideoPrompt>,
    trace: VideoPromptGenerationTrace,
) -> Vec<NewChapterVideoPrompt> {
    drafts
        .into_iter()
        .map(|draft| draft.with_generation_trace(trace.clone()))
        .collect()
}

pub(super) fn video_prompt_generation_trace(
    trace: RenderedVideoPromptTrace,
    workflow_run_id: Option<i64>,
) -> VideoPromptGenerationTrace {
    VideoPromptGenerationTrace {
        prompt_name: trace.prompt_name,
        request: trace.request,
        response_text: trace.response_text,
        workflow_run_id,
        usage: trace.usage,
    }
}

async fn render_json_with_validation_with_context<T, F>(
    input: VideoPromptRenderInput<'_, '_, '_, '_>,
    parse_and_validate: F,
) -> Result<(RenderedVideoPromptTrace, T), AppError>
where
    F: Fn(&str) -> Result<T, AppError>,
{
    let trace = render_and_chat_trace_with_context(input).await?;
    let parsed = parse_and_validate(&trace.response_text)?;
    Ok((trace, parsed))
}

pub(in crate::video::prompt_generation::frame_prompts) struct RenderOrRestoreJsonWithValidationInput<
    'a,
    'step,
    'progress,
    'chunk,
> {
    pub service: &'a VideoService,
    pub workflow_run_id: i64,
    pub call_key: String,
    pub call_order: i32,
    pub input_fingerprint_json: Value,
    pub prompt_engine: &'a dyn PromptEngine,
    pub llm: &'a dyn LlmProvider,
    pub prompt_name: &'a str,
    pub project_id: i64,
    pub variables: &'a HashMap<String, Value>,
    pub context_manifest: LlmContextManifest,
    pub temperature: f32,
    pub cancel_token: Option<&'a CancellationToken>,
    pub step: VideoPromptStepProgress<'step>,
    pub progress: &'a mut Option<VideoPromptProgress<'progress>>,
    pub chunks: &'a mut Option<VideoPromptChunk<'chunk>>,
    pub structured_progress: Option<StructuredJsonProgressSpec>,
    pub checkpoint_context: &'a str,
}

pub(in crate::video::prompt_generation::frame_prompts) async fn render_or_restore_json_with_validation_with_context<
    T,
    F,
    G,
>(
    input: RenderOrRestoreJsonWithValidationInput<'_, '_, '_, '_>,
    parse_and_validate: F,
    checkpoint_parsed_output: G,
) -> Result<(RenderedVideoPromptTrace, T), AppError>
where
    F: Fn(&str) -> Result<T, AppError>,
    G: Fn(&T) -> Result<Value, AppError>,
{
    let RenderOrRestoreJsonWithValidationInput {
        service,
        workflow_run_id,
        call_key,
        call_order,
        input_fingerprint_json,
        prompt_engine,
        llm,
        prompt_name,
        project_id,
        variables,
        context_manifest,
        temperature,
        cancel_token,
        step,
        progress,
        chunks,
        structured_progress,
        checkpoint_context,
    } = input;
    let llm_call_trace =
        video_llm_call_trace_context(workflow_run_id, &call_key, &input_fingerprint_json)?;
    if let Some(checkpoint) = load_completed_video_checkpoint(
        service,
        workflow_run_id,
        &call_key,
        &input_fingerprint_json,
    )
    .await?
    {
        let response_content =
            checkpoint_response_content(&checkpoint, checkpoint_context)?.to_string();
        let request = build_video_prompt_request_with_call_trace(
            prompt_engine,
            prompt_name,
            project_id,
            variables,
            context_manifest.clone(),
            temperature,
            llm_call_trace.clone(),
        )?;
        let parsed = match parse_and_validate(&response_content) {
            Ok(parsed) => parsed,
            Err(err) => {
                let diagnostic = save_failed_video_llm_checkpoint(
                    service,
                    FailedVideoLlmCheckpointInput {
                        workflow_run_id,
                        call_key,
                        call_order,
                        input_fingerprint_json,
                        prompt_name,
                        request: &request,
                        response_content: Some(response_content.as_str()),
                        error_message: err.to_string(),
                        source: "completed_checkpoint_restore",
                    },
                )
                .await;
                return Err(video_llm_validation_error_with_diagnostic(err, diagnostic));
            }
        };
        let parsed_output = checkpoint_parsed_output(&parsed)?;
        if let Err(err) =
            ensure_checkpoint_parsed_output(&checkpoint, &parsed_output, checkpoint_context)
        {
            let diagnostic = save_failed_video_llm_checkpoint(
                service,
                FailedVideoLlmCheckpointInput {
                    workflow_run_id,
                    call_key,
                    call_order,
                    input_fingerprint_json,
                    prompt_name,
                    request: &request,
                    response_content: Some(response_content.as_str()),
                    error_message: err.to_string(),
                    source: "completed_checkpoint_restore",
                },
            )
            .await;
            return Err(video_llm_validation_error_with_diagnostic(err, diagnostic));
        }
        let usage = checkpoint_usage(&checkpoint, checkpoint_context)?;
        let finish_reason = checkpoint_finish_reason(&checkpoint, checkpoint_context)?;
        report_video_prompt_progress(
            progress,
            step.current,
            step.total,
            &format!("{}（已复用调用级断点）", step.message),
        )?;
        return Ok((
            RenderedVideoPromptTrace {
                prompt_name: prompt_name.to_string(),
                request,
                response_text: response_content,
                usage,
                finish_reason,
            },
            parsed,
        ));
    }

    let trace = render_and_chat_trace_with_context(VideoPromptRenderInput {
        prompt_engine,
        llm,
        prompt_name,
        project_id,
        variables,
        context_manifest,
        temperature,
        llm_call_trace,
        cancel_token,
        step,
        progress,
        chunks,
        structured_progress,
    })
    .await?;
    let parsed = match parse_and_validate(&trace.response_text) {
        Ok(parsed) => parsed,
        Err(err) => {
            let diagnostic = save_failed_video_llm_checkpoint(
                service,
                FailedVideoLlmCheckpointInput {
                    workflow_run_id,
                    call_key,
                    call_order,
                    input_fingerprint_json,
                    prompt_name,
                    request: &trace.request,
                    response_content: Some(trace.response_text.as_str()),
                    error_message: err.to_string(),
                    source: "llm_response_validation",
                },
            )
            .await;
            return Err(video_llm_validation_error_with_diagnostic(err, diagnostic));
        }
    };
    let parsed_output = checkpoint_parsed_output(&parsed)?;
    save_completed_video_llm_checkpoint(CompletedVideoLlmCheckpointInput {
        service,
        workflow_run_id,
        step_key: call_key,
        step_order: call_order,
        input_fingerprint_json,
        trace: &trace,
        parsed_output,
        extra_json: json!({}),
        artifacts_json: None,
    })
    .await?;
    Ok((trace, parsed))
}

const VIDEO_BEAT_EXTRACTION_SPAN_WINDOW: usize = 48;

fn source_span_window_content(source_spans: &[SourceSpan]) -> String {
    source_spans
        .iter()
        .map(|span| span.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(in crate::video) fn source_span_windows(source_spans: &[SourceSpan]) -> Vec<&[SourceSpan]> {
    let mut windows = Vec::new();
    let mut start = 0usize;
    while start < source_spans.len() {
        let mut end = start;
        let mut span_count = 0usize;
        while end < source_spans.len() {
            let paragraph_index = source_spans[end].paragraph_index;
            let paragraph_start = end;
            while end < source_spans.len() && source_spans[end].paragraph_index == paragraph_index {
                end += 1;
            }
            let paragraph_len = end - paragraph_start;
            if paragraph_len > VIDEO_BEAT_EXTRACTION_SPAN_WINDOW {
                if span_count == 0 {
                    end = (paragraph_start + VIDEO_BEAT_EXTRACTION_SPAN_WINDOW).min(end);
                } else {
                    end = paragraph_start;
                }
                break;
            }
            if span_count > 0 && span_count + paragraph_len > VIDEO_BEAT_EXTRACTION_SPAN_WINDOW {
                end = paragraph_start;
                break;
            }
            span_count += paragraph_len;
            if span_count >= VIDEO_BEAT_EXTRACTION_SPAN_WINDOW {
                break;
            }
        }
        if end == start {
            end = (start + VIDEO_BEAT_EXTRACTION_SPAN_WINDOW).min(source_spans.len());
        }
        windows.push(&source_spans[start..end]);
        start = end;
    }
    windows
}

fn video_beats_for_prompt(beats: &[VideoActionBeat], source_spans: &[SourceSpan]) -> Value {
    let source_text_by_id = source_spans
        .iter()
        .map(|span| (span.id.as_str(), span.text.as_str()))
        .collect::<HashMap<_, _>>();
    Value::Array(
        beats
            .iter()
            .map(|beat| {
                let source_texts = beat
                    .source_span_ids
                    .iter()
                    .filter_map(|id| source_text_by_id.get(id.as_str()).copied())
                    .collect::<Vec<_>>();
                json!({
                    "beat_id": beat.beat_id,
                    "beat_type": beat.beat_type,
                    "natural_seconds": beat.natural_seconds,
                    "required": beat.required,
                    "visible_action": beat.visible_action,
                    "source_span_ids": beat.source_span_ids,
                    "source_texts": source_texts
                })
            })
            .collect(),
    )
}

fn source_spans_for_shot(
    source_spans: &[SourceSpan],
    shot: &VideoCinematicShot,
) -> Vec<SourceSpan> {
    let ids = shot
        .source_span_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    source_spans
        .iter()
        .filter(|span| ids.contains(span.id.as_str()))
        .cloned()
        .collect()
}

fn serialize_video_value<T: serde::Serialize>(value: &T, context: &str) -> Result<Value, AppError> {
    serde_json::to_value(value)
        .map_err(|err| AppError::InvalidArgument(format!("{context} 序列化失败: {err}")))
}

fn video_stage_fingerprint(
    prompt_name: &str,
    context_manifest: &LlmContextManifest,
    extra: Value,
) -> Value {
    json!({
        "schema_version": 1,
        "prompt_name": prompt_name,
        "context_manifest": context_manifest,
        "extra": extra,
    })
}

fn video_final_segment_plan_fingerprint(
    project_id: i64,
    source: ChapterVideoSourceContext<'_>,
    shot_analysis_json: &Value,
    style: &str,
    duration_secs: i32,
    aspect_ratio: &str,
) -> Value {
    json!({
        "schema_version": 1,
        "kind": "video_segment_plan.final",
        "project_id": project_id,
        "source_sha256": source_content_sha256(source.content),
        "source_summary": source.summary,
        "source_analysis": source.analysis,
        "shot_analysis": shot_analysis_json,
        "style": style,
        "duration_secs": duration_secs,
        "aspect_ratio": aspect_ratio,
    })
}

async fn load_completed_video_checkpoint(
    service: &VideoService,
    workflow_run_id: i64,
    step_key: &str,
    input_fingerprint_json: &Value,
) -> Result<Option<Value>, AppError> {
    let Some(checkpoint) = load_completed_llm_call_checkpoint(
        service.db.clone(),
        workflow_run_id,
        step_key,
        input_fingerprint_json,
    )
    .await?
    else {
        return Ok(None);
    };
    let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, step_key)?;
    Ok(Some(checkpoint_json))
}

async fn load_completed_video_workflow_checkpoint(
    service: &VideoService,
    workflow_run_id: i64,
    step_key: &str,
    input_fingerprint_json: &Value,
) -> Result<Option<Value>, AppError> {
    let input_fingerprint_sha256 =
        generation_checkpoint_fingerprint_sha256(input_fingerprint_json)?;
    WorkflowRunService::new(service.db.clone())
        .completed_checkpoint_by_fingerprint(workflow_run_id, step_key, &input_fingerprint_sha256)
        .await?
        .map(|checkpoint| {
            checkpoint
                .checkpoint_json
                .ok_or_else(|| AppError::InvalidState {
                    current: "missing_checkpoint_json".to_string(),
                    operation: format!("读取视频工作流断点 {step_key}"),
                })
        })
        .transpose()
}

async fn save_completed_video_workflow_checkpoint(
    service: &VideoService,
    workflow_run_id: i64,
    step_key: String,
    step_order: i32,
    input_fingerprint_json: Value,
    checkpoint_json: Value,
    artifacts_json: Option<Value>,
) -> Result<(), AppError> {
    let input_fingerprint_sha256 =
        generation_checkpoint_fingerprint_sha256(&input_fingerprint_json)?;
    WorkflowRunService::new(service.db.clone())
        .upsert_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id,
            step_key,
            step_order,
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            input_fingerprint_sha256: Some(input_fingerprint_sha256),
            input_fingerprint_json: Some(input_fingerprint_json),
            checkpoint_json: Some(checkpoint_json),
            artifacts_json,
            generation_run_ids_json: None,
            error_message: None,
            completed: true,
        })
        .await?;
    Ok(())
}

struct CompletedVideoLlmCheckpointInput<'a> {
    service: &'a VideoService,
    workflow_run_id: i64,
    step_key: String,
    step_order: i32,
    input_fingerprint_json: Value,
    trace: &'a RenderedVideoPromptTrace,
    parsed_output: Value,
    extra_json: Value,
    artifacts_json: Option<Value>,
}

async fn save_completed_video_llm_checkpoint(
    input: CompletedVideoLlmCheckpointInput<'_>,
) -> Result<(), AppError> {
    let CompletedVideoLlmCheckpointInput {
        service,
        workflow_run_id,
        step_key,
        step_order,
        input_fingerprint_json,
        trace,
        parsed_output,
        extra_json,
        artifacts_json,
    } = input;
    let usage = trace.usage.as_ref().map(serde_json::to_value).transpose()?;
    let mut checkpoint_json = extra_json.as_object().cloned().ok_or_else(|| {
        AppError::InvalidArgument("视频 LLM checkpoint 扩展字段必须是 JSON 对象".to_string())
    })?;
    checkpoint_json.insert("output".to_string(), parsed_output.clone());
    checkpoint_json.insert("parsed_output".to_string(), parsed_output);
    checkpoint_json.insert(
        "response_content".to_string(),
        Value::String(trace.response_text.clone()),
    );
    checkpoint_json.insert("usage".to_string(), usage.unwrap_or(Value::Null));
    checkpoint_json.insert(
        "finish_reason".to_string(),
        trace
            .finish_reason
            .as_ref()
            .map(|value| Value::String(value.clone()))
            .unwrap_or(Value::Null),
    );
    save_completed_llm_call_checkpoint(
        service.db.clone(),
        CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key: step_key,
            call_order: step_order,
            input_fingerprint_json,
            checkpoint_json: Value::Object(checkpoint_json),
            artifacts_json,
        },
    )
    .await
}

struct FailedVideoLlmCheckpointInput<'a> {
    workflow_run_id: i64,
    call_key: String,
    call_order: i32,
    input_fingerprint_json: Value,
    prompt_name: &'a str,
    request: &'a ChatRequest,
    response_content: Option<&'a str>,
    error_message: String,
    source: &'a str,
}

async fn save_failed_video_llm_checkpoint(
    service: &VideoService,
    input: FailedVideoLlmCheckpointInput<'_>,
) -> Result<String, AppError> {
    let input_fingerprint_sha256 =
        llm_call_checkpoint_fingerprint_sha256(&input.input_fingerprint_json)?;
    let request_json = context_aware_chat_request_trace_json(input.request)?;
    WorkflowRunService::new(service.db.clone())
        .upsert_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id: input.workflow_run_id,
            step_key: input.call_key.clone(),
            step_order: input.call_order,
            status: WORKFLOW_STATUS_FAILED.to_string(),
            input_fingerprint_sha256: Some(input_fingerprint_sha256.clone()),
            input_fingerprint_json: Some(input.input_fingerprint_json),
            checkpoint_json: Some(json!({
                "diagnostic_schema_version": 1,
                "diagnostic_kind": "video_llm_response_validation_failed",
                "source": input.source,
                "prompt_name": input.prompt_name,
                "call_key": input.call_key.clone(),
                "request": request_json,
                "response_content": input.response_content,
                "error_message": input.error_message.clone(),
            })),
            artifacts_json: Some(json!({
                "request_saved": true,
                "response_saved": input.response_content.is_some(),
            })),
            generation_run_ids_json: None,
            error_message: Some(input.error_message.clone()),
            completed: false,
        })
        .await?;
    Ok(format!(
        "已保存 LLM 请求/响应诊断：workflow_run_id={}, step_key={}, input_fingerprint_sha256={}",
        input.workflow_run_id, input.call_key, input_fingerprint_sha256
    ))
}

fn video_llm_validation_error_with_diagnostic(
    err: AppError,
    diagnostic: Result<String, AppError>,
) -> AppError {
    let suffix = match diagnostic {
        Ok(message) => message,
        Err(save_err) => format!("LLM 请求/响应诊断保存失败：{save_err}"),
    };
    match err {
        AppError::InvalidArgument(message) => {
            AppError::InvalidArgument(format!("{message}；{suffix}"))
        }
        AppError::InvalidState { current, operation } => AppError::InvalidState {
            current,
            operation: format!("{operation}；{suffix}"),
        },
        AppError::Timeout(message) => AppError::Timeout(format!("{message}；{suffix}")),
        AppError::LlmError(message) => AppError::LlmError(format!("{message}；{suffix}")),
        other => AppError::InvalidArgument(format!("{other}；{suffix}")),
    }
}

fn video_llm_call_trace_context(
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<VideoPromptLlmCallTraceContext, AppError> {
    Ok(VideoPromptLlmCallTraceContext {
        workflow_run_id,
        call_key: call_key.to_string(),
        input_fingerprint_sha256: llm_call_checkpoint_fingerprint_sha256(input_fingerprint_json)?,
    })
}

fn checkpoint_response_content<'a>(
    checkpoint: &'a Value,
    context: &str,
) -> Result<&'a str, AppError> {
    checkpoint
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_response_content".to_string(),
            operation: format!("读取{context}"),
        })
}

fn ensure_checkpoint_parsed_output(
    checkpoint: &Value,
    parsed_output: &Value,
    context: &str,
) -> Result<(), AppError> {
    let checkpoint_output =
        checkpoint
            .get("parsed_output")
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_checkpoint_parsed_output".to_string(),
                operation: format!("读取{context}"),
            })?;
    if checkpoint_output != parsed_output {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_mismatch".to_string(),
            operation: format!("校验{context}"),
        });
    }
    Ok(())
}

fn checkpoint_usage(checkpoint: &Value, context: &str) -> Result<Option<UsageInfo>, AppError> {
    checkpoint_field(checkpoint, "usage", context)
}

fn checkpoint_finish_reason(checkpoint: &Value, context: &str) -> Result<Option<String>, AppError> {
    checkpoint_field(checkpoint, "finish_reason", context)
}

fn checkpoint_field<T: DeserializeOwned>(
    checkpoint: &Value,
    key: &str,
    context: &str,
) -> Result<T, AppError> {
    let value = checkpoint
        .get(key)
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_field".to_string(),
            operation: format!("{context} 缺少字段 {key}"),
        })?;
    serde_json::from_value(value)
        .map_err(|err| AppError::InvalidArgument(format!("{context}.{key} 解析失败: {err}")))
}

struct StagedShotAnalysisInput<'a, 'source, 'progress, 'chunk> {
    service: &'a VideoService,
    prompt_engine: &'a dyn PromptEngine,
    llm: &'a dyn LlmProvider,
    project_id: i64,
    workflow_run_id: Option<i64>,
    variables: &'a HashMap<String, Value>,
    source: ChapterVideoSourceContext<'source>,
    source_spans: &'a [SourceSpan],
    project_context: Option<&'a Value>,
    style: &'a str,
    duration_secs: i32,
    aspect_ratio: &'a str,
    progress_message: &'a str,
    progress_span: ProgressSpan,
    cancel_token: Option<&'a CancellationToken>,
    progress: &'a mut Option<VideoPromptProgress<'progress>>,
    chunks: &'a mut Option<VideoPromptChunk<'chunk>>,
}

async fn render_staged_shot_analysis(
    input: StagedShotAnalysisInput<'_, '_, '_, '_>,
) -> Result<
    (
        Option<RenderedVideoPromptTrace>,
        Value,
        VideoShotAnalysis,
        Value,
    ),
    AppError,
> {
    let StagedShotAnalysisInput {
        service,
        prompt_engine,
        llm,
        project_id,
        workflow_run_id,
        variables,
        source,
        source_spans,
        project_context,
        style,
        duration_secs,
        aspect_ratio,
        progress_message,
        progress_span,
        cancel_token,
        progress,
        chunks,
    } = input;
    let workflow_run_id =
        required_llm_call_workflow_run_id(workflow_run_id, "video_shot_analysis")?;
    let windows = source_span_windows(source_spans);
    if windows.is_empty() {
        return Err(AppError::InvalidArgument(
            "章节正文无法切分出可用于视频镜头分析的 source_spans".to_string(),
        ));
    }
    let beat_span = progress_span.first_half();
    let grouping_span = progress_span.second_half();
    let planning_prompt_context = VideoPlanningPromptContext {
        prompt_engine,
        project_id,
        chapter_summary: source.summary,
        chapter_analysis: source.analysis,
        project_context,
        style,
        duration_secs,
        aspect_ratio,
    };
    let mut beats = Vec::<VideoActionBeat>::new();
    let mut beat_stage_outputs = Vec::new();
    let window_count = windows.len();
    for (index, window) in windows.iter().enumerate() {
        let window_number = index + 1;
        let first_beat_id = beats.len() as i32 + 1;
        let user_content = build_beat_extraction_user(
            planning_prompt_context,
            window,
            window_number,
            window_count,
        )?;
        let window_context = format!("视频动作节拍抽取窗口 {window_number}/{window_count}");
        let window_span = beat_span.split(index, window_count);
        let window_progress_message =
            format!("{progress_message}：抽取动作节拍窗口 {window_number}/{window_count}");
        report_video_prompt_progress(progress, window_span.start(), 100, &window_progress_message)?;
        let window_content = source_span_window_content(window);
        let window_source = ChapterVideoSourceContext {
            content: &window_content,
            summary: source.summary,
            analysis: source.analysis,
        };
        let context_manifest = build_video_context_manifest(VideoContextManifestInput {
            prompt_engine,
            project_id,
            variables,
            source: &window_source,
            source_spans: window,
            prompt_names: &["chapter_video_beat_extraction"],
            previous_outputs: &[],
            project_context,
            asset_context: None,
            dynamic_user_content: user_content.clone(),
        })?;
        let step_key = format!("video_shot_analysis.beat_window.{window_number}");
        let fingerprint_json = video_stage_fingerprint(
            "chapter_video_beat_extraction",
            &context_manifest,
            json!({
                "project_id": project_id,
                "window_number": window_number,
                "window_count": window_count,
                "first_beat_id": first_beat_id,
                "source_span_ids": window.iter().map(|span| span.id.as_str()).collect::<Vec<_>>(),
            }),
        );
        if let Some(checkpoint) =
            load_completed_video_checkpoint(service, workflow_run_id, &step_key, &fingerprint_json)
                .await?
        {
            let response_content = checkpoint_response_content(&checkpoint, &window_context)?;
            let window_beats = parse_video_beat_extraction_response(
                response_content,
                window,
                first_beat_id,
                &window_context,
            )?;
            let parsed_output = serde_json::to_value(&window_beats)?;
            ensure_checkpoint_parsed_output(&checkpoint, &parsed_output, &window_context)?;
            let stage_output = json!({
                "window_index": window_number,
                "window_count": window_count,
                "prompt_name": "chapter_video_beat_extraction",
                "source_span_start": window.first().map(|span| span.id.as_str()),
                "source_span_end": window.last().map(|span| span.id.as_str()),
                "raw": response_content
            });
            beat_stage_outputs.push(stage_output);
            beats.extend(window_beats);
            report_video_prompt_progress(
                progress,
                window_span.end(),
                100,
                &format!("{window_progress_message}（已复用断点）"),
            )?;
            continue;
        }
        let (trace, window_beats) = render_json_with_validation_with_context(
            VideoPromptRenderInput {
                prompt_engine,
                llm,
                prompt_name: "chapter_video_beat_extraction",
                project_id,
                variables,
                context_manifest,
                temperature: 0.25,
                llm_call_trace: video_llm_call_trace_context(
                    workflow_run_id,
                    &step_key,
                    &fingerprint_json,
                )?,
                cancel_token,
                step: VideoPromptStepProgress::new(
                    window_span.start(),
                    100,
                    &window_progress_message,
                )
                .with_span(window_span),
                progress,
                chunks,
                structured_progress: Some(beat_extraction_progress_spec(None)),
            },
            |raw| parse_video_beat_extraction_response(raw, window, first_beat_id, &window_context),
        )
        .await?;
        let stage_output = json!({
            "window_index": window_number,
            "window_count": window_count,
            "prompt_name": trace.prompt_name.clone(),
            "source_span_start": window.first().map(|span| span.id.as_str()),
            "source_span_end": window.last().map(|span| span.id.as_str()),
            "raw": trace.response_text.clone()
        });
        save_completed_video_llm_checkpoint(CompletedVideoLlmCheckpointInput {
            service,
            workflow_run_id,
            step_key,
            step_order: 10 + i32::try_from(window_number).unwrap_or(i32::MAX),
            input_fingerprint_json: fingerprint_json,
            trace: &trace,
            parsed_output: serde_json::to_value(&window_beats)?,
            extra_json: json!({
                "stage_output": stage_output.clone(),
            }),
            artifacts_json: None,
        })
        .await?;
        beat_stage_outputs.push(stage_output);
        beats.extend(window_beats);
    }
    if beats.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频动作节拍抽取结果不能为空".to_string(),
        ));
    }

    let extracted_beats = video_beats_for_prompt(&beats, source_spans);
    let grouping_user = build_shot_grouping_user(planning_prompt_context, &extracted_beats)?;
    let beat_context_output = json!({
        "parsed_beats": extracted_beats,
        "stage_outputs": &beat_stage_outputs
    });
    let grouping_context_manifest = build_video_context_manifest(VideoContextManifestInput {
        prompt_engine,
        project_id,
        variables,
        source: &source,
        source_spans,
        prompt_names: &[
            "chapter_video_beat_extraction",
            "chapter_video_shot_analysis",
        ],
        previous_outputs: &[VideoContextPreviousOutput {
            pack_kind: VIDEO_PREVIOUS_OUTPUT_BEAT_EXTRACTION,
            prompt_name: "chapter_video_beat_extraction",
            parsed_output: &beat_context_output,
            raw_output: None,
            placeholder: false,
        }],
        project_context,
        asset_context: None,
        dynamic_user_content: grouping_user.clone(),
    })?;
    let grouping_fingerprint_json = video_stage_fingerprint(
        "chapter_video_shot_analysis",
        &grouping_context_manifest,
        json!({
            "project_id": project_id,
            "beat_count": beats.len(),
        }),
    );
    if let Some(checkpoint) = load_completed_video_checkpoint(
        service,
        workflow_run_id,
        "video_shot_analysis.grouping",
        &grouping_fingerprint_json,
    )
    .await?
    {
        let response_content = checkpoint_response_content(&checkpoint, "视频镜头分组断点")?;
        let analysis = parse_video_shot_grouping_response(
            response_content,
            source_spans,
            duration_secs,
            &beats,
        )?;
        let analysis_json = serde_json::to_value(&analysis).map_err(|err| {
            AppError::InvalidArgument(format!(
                "chapter_video_shot_analysis 派生结果序列化失败: {err}"
            ))
        })?;
        ensure_checkpoint_parsed_output(&checkpoint, &analysis_json, "视频镜头分组断点")?;
        let stage_outputs = json!({
            "beat_extraction": beat_stage_outputs,
            "shot_grouping": {
                "prompt_name": "chapter_video_shot_analysis",
                "raw": response_content
            }
        });
        report_video_prompt_progress(
            progress,
            grouping_span.end(),
            100,
            &format!("{progress_message}：合并镜头分组（已复用断点）"),
        )?;
        return Ok((None, analysis_json, analysis, stage_outputs));
    }
    let grouping_progress_message = format!("{progress_message}：合并镜头分组");
    report_video_prompt_progress(
        progress,
        grouping_span.start(),
        100,
        &grouping_progress_message,
    )?;
    let (grouping_trace, (analysis_json, analysis)) = render_json_with_validation_with_context(
        VideoPromptRenderInput {
            prompt_engine,
            llm,
            prompt_name: "chapter_video_shot_analysis",
            project_id,
            variables,
            context_manifest: grouping_context_manifest,
            temperature: 0.3,
            llm_call_trace: video_llm_call_trace_context(
                workflow_run_id,
                "video_shot_analysis.grouping",
                &grouping_fingerprint_json,
            )?,
            cancel_token,
            step: VideoPromptStepProgress::new(
                grouping_span.start(),
                100,
                &grouping_progress_message,
            )
            .with_span(grouping_span),
            progress,
            chunks,
            structured_progress: Some(shot_analysis_progress_spec(None)),
        },
        |raw| {
            let analysis =
                parse_video_shot_grouping_response(raw, source_spans, duration_secs, &beats)?;
            let analysis_json = serde_json::to_value(&analysis).map_err(|err| {
                AppError::InvalidArgument(format!(
                    "chapter_video_shot_analysis 派生结果序列化失败: {err}"
                ))
            })?;
            Ok((analysis_json, analysis))
        },
    )
    .await?;
    let stage_outputs = json!({
        "beat_extraction": beat_stage_outputs,
        "shot_grouping": {
            "prompt_name": grouping_trace.prompt_name,
            "raw": grouping_trace.response_text
        }
    });
    save_completed_video_llm_checkpoint(CompletedVideoLlmCheckpointInput {
        service,
        workflow_run_id,
        step_key: "video_shot_analysis.grouping".to_string(),
        step_order: 100,
        input_fingerprint_json: grouping_fingerprint_json,
        trace: &grouping_trace,
        parsed_output: analysis_json.clone(),
        extra_json: json!({
            "stage_outputs": stage_outputs.clone(),
        }),
        artifacts_json: None,
    })
    .await?;
    Ok((Some(grouping_trace), analysis_json, analysis, stage_outputs))
}

struct StagedSegmentPlanInput<'a, 'source, 'progress, 'chunk> {
    service: &'a VideoService,
    prompt_engine: &'a dyn PromptEngine,
    llm: &'a dyn LlmProvider,
    project_id: i64,
    workflow_run_id: Option<i64>,
    variables: &'a HashMap<String, Value>,
    source: ChapterVideoSourceContext<'source>,
    source_spans: &'a [SourceSpan],
    project_context: Option<&'a Value>,
    shot_analysis_json: &'a Value,
    shot_analysis: &'a VideoShotAnalysis,
    default_aspect_ratio: &'a str,
    style: &'a str,
    duration_secs: i32,
    context: &'a str,
    progress_message: &'a str,
    progress_span: ProgressSpan,
    cancel_token: Option<&'a CancellationToken>,
    progress: &'a mut Option<VideoPromptProgress<'progress>>,
    chunks: &'a mut Option<VideoPromptChunk<'chunk>>,
}

async fn render_staged_segment_plan_with_duration(
    input: StagedSegmentPlanInput<'_, '_, '_, '_>,
) -> Result<
    (
        Option<RenderedVideoPromptTrace>,
        Value,
        Vec<VideoSegmentPlan>,
        Value,
    ),
    AppError,
> {
    let StagedSegmentPlanInput {
        service,
        prompt_engine,
        llm,
        project_id,
        workflow_run_id,
        variables,
        source,
        source_spans,
        project_context,
        shot_analysis_json,
        shot_analysis,
        default_aspect_ratio,
        style,
        duration_secs,
        context,
        progress_message,
        progress_span,
        cancel_token,
        progress,
        chunks,
    } = input;
    let workflow_run_id = required_llm_call_workflow_run_id(workflow_run_id, "video_segment_plan")?;
    if shot_analysis.cinematic_shots.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频片段规划必须基于至少一个导演镜头".to_string(),
        ));
    }
    let final_fingerprint_json = video_final_segment_plan_fingerprint(
        project_id,
        source,
        shot_analysis_json,
        style,
        duration_secs,
        default_aspect_ratio,
    );
    if let Some(checkpoint) = load_completed_video_workflow_checkpoint(
        service,
        workflow_run_id,
        "video_segment_plan.final",
        &final_fingerprint_json,
    )
    .await?
    {
        let segment_json: Value =
            checkpoint_field(&checkpoint, "segment_json", "视频片段规划 final 断点")?;
        let segment_plan: Vec<VideoSegmentPlan> =
            checkpoint_field(&checkpoint, "segment_plan", "视频片段规划 final 断点")?;
        let stage_outputs: Value =
            checkpoint_field(&checkpoint, "stage_outputs", "视频片段规划 final 断点")?;
        validate_segment_plan_against_shot_analysis(&segment_plan, shot_analysis)?;
        report_video_prompt_progress(
            progress,
            progress_span.end(),
            100,
            &format!("{progress_message}（已复用断点）"),
        )?;
        return Ok((None, segment_json, segment_plan, stage_outputs));
    }
    let total_shots = shot_analysis.cinematic_shots.len();
    let planning_prompt_context = VideoPlanningPromptContext {
        prompt_engine,
        project_id,
        chapter_summary: source.summary,
        chapter_analysis: source.analysis,
        project_context,
        style,
        duration_secs,
        aspect_ratio: default_aspect_ratio,
    };
    let mut all_segment_inputs = Vec::<Value>::new();
    let mut stage_outputs = Vec::new();
    let mut last_trace = None;
    for (index, shot) in shot_analysis.cinematic_shots.iter().enumerate() {
        let shot_value = serialize_video_value(shot, "当前导演镜头")?;
        let shot_source_spans = source_spans_for_shot(source_spans, shot);
        if shot_source_spans.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "视频片段规划镜头 #{} 缺少可用于提示词的 source_spans",
                shot.shot_id
            )));
        }
        let user_content = build_segment_plan_for_shot_user(
            planning_prompt_context,
            shot_analysis_json,
            &shot_value,
            &shot_source_spans,
        )?;
        let shot_context = format!("{context} 镜头 #{}", shot.shot_id);
        let shot_span = progress_span.split(index, total_shots);
        let expected_items = Some(shot.required_segment_count.max(1) as usize);
        let context_manifest = build_video_context_manifest(VideoContextManifestInput {
            prompt_engine,
            project_id,
            variables,
            source: &source,
            source_spans,
            prompt_names: &["chapter_video_shot_analysis", "chapter_video_segment_plan"],
            previous_outputs: &[VideoContextPreviousOutput {
                pack_kind: VIDEO_PREVIOUS_OUTPUT_SHOT_ANALYSIS,
                prompt_name: "chapter_video_shot_analysis",
                parsed_output: shot_analysis_json,
                raw_output: None,
                placeholder: false,
            }],
            project_context,
            asset_context: None,
            dynamic_user_content: user_content.clone(),
        })?;
        let step_key = format!("video_segment_plan.shot.{}", shot.shot_id);
        let fingerprint_json = video_stage_fingerprint(
            "chapter_video_segment_plan",
            &context_manifest,
            json!({
                "project_id": project_id,
                "shot_id": shot.shot_id,
                "shot_index": index + 1,
                "total_shots": total_shots,
                "expected_items": expected_items,
            }),
        );
        if let Some(checkpoint) =
            load_completed_video_checkpoint(service, workflow_run_id, &step_key, &fingerprint_json)
                .await?
        {
            let response_content = checkpoint_response_content(&checkpoint, &shot_context)?;
            let segment_values =
                parse_video_segment_plan_input_values(response_content, &shot_context)?;
            let parsed_output = serde_json::to_value(&segment_values)?;
            ensure_checkpoint_parsed_output(&checkpoint, &parsed_output, &shot_context)?;
            let stage_output = json!({
                "shot_id": shot.shot_id,
                "prompt_name": "chapter_video_segment_plan",
                "raw": response_content
            });
            stage_outputs.push(stage_output);
            all_segment_inputs.extend(segment_values);
            report_video_prompt_progress(
                progress,
                shot_span.end(),
                100,
                &format!("{progress_message}（镜头 #{} 已复用断点）", shot.shot_id),
            )?;
            continue;
        }
        let (trace, segment_values) = render_json_with_validation_with_context(
            VideoPromptRenderInput {
                prompt_engine,
                llm,
                prompt_name: "chapter_video_segment_plan",
                project_id,
                variables,
                context_manifest,
                temperature: 0.35,
                llm_call_trace: video_llm_call_trace_context(
                    workflow_run_id,
                    &step_key,
                    &fingerprint_json,
                )?,
                cancel_token,
                step: VideoPromptStepProgress::new(shot_span.start(), 100, progress_message)
                    .with_span(shot_span),
                progress,
                chunks,
                structured_progress: Some(segment_plan_progress_spec(expected_items)),
            },
            |raw| parse_video_segment_plan_input_values(raw, &shot_context),
        )
        .await?;
        let stage_output = json!({
            "shot_id": shot.shot_id,
            "prompt_name": trace.prompt_name.clone(),
            "raw": trace.response_text.clone()
        });
        save_completed_video_llm_checkpoint(CompletedVideoLlmCheckpointInput {
            service,
            workflow_run_id,
            step_key,
            step_order: 200 + i32::try_from(index).unwrap_or(i32::MAX),
            input_fingerprint_json: fingerprint_json,
            trace: &trace,
            parsed_output: serde_json::to_value(&segment_values)?,
            extra_json: json!({
                "stage_output": stage_output.clone(),
            }),
            artifacts_json: None,
        })
        .await?;
        stage_outputs.push(stage_output);
        all_segment_inputs.extend(segment_values);
        last_trace = Some(trace);
    }
    let combined_raw = json!({ "segments": all_segment_inputs }).to_string();
    let (segment_json, segment_plan) = parse_generated_segment_plan_with_current_config(
        &combined_raw,
        source_spans,
        shot_analysis,
        duration_secs,
        default_aspect_ratio,
    )?;
    validate_segment_plan_against_shot_analysis(&segment_plan, shot_analysis)?;
    save_completed_video_workflow_checkpoint(
        service,
        workflow_run_id,
        "video_segment_plan.final".to_string(),
        299,
        final_fingerprint_json,
        json!({
            "segment_json": segment_json.clone(),
            "segment_plan": segment_plan.clone(),
            "stage_outputs": json!({ "segment_plan_per_shot": stage_outputs.clone() }),
        }),
        None,
    )
    .await?;
    Ok((
        last_trace,
        segment_json,
        segment_plan,
        json!({ "segment_plan_per_shot": stage_outputs }),
    ))
}

struct PreparedVideoPromptInputs {
    source_metadata: Value,
    source_spans: Vec<SourceSpan>,
    regenerate_existing: bool,
    prompt_batch_size: usize,
    style: String,
    prompt_output_language: PromptOutputLanguage,
    duration_secs: i32,
    aspect_ratio: String,
    reference_image_ids: Vec<i64>,
    video_library_image_ids: Vec<i64>,
    reference_ids: Value,
    generation_asset_fingerprint: Value,
    project_context: Option<Value>,
    available_assets: Value,
    vars: HashMap<String, Value>,
    workflow_run_id: Option<i64>,
}

struct PreparedShotAnalysis {
    analysis_json: Value,
    analysis: VideoShotAnalysis,
    draft: Option<NewChapterVideoPrompt>,
}

#[derive(Debug, Clone, Copy)]
struct SharedShotAnalysisSpec {
    workflow_mode: PromptWorkflowMode,
    prompt_type: &'static str,
    title: &'static str,
}

const SHARED_SHOT_ANALYSIS_SPECS: &[SharedShotAnalysisSpec] = &[SharedShotAnalysisSpec {
    workflow_mode: SHARED_SHOT_ANALYSIS_WORKFLOW_MODE,
    prompt_type: SHARED_SHOT_ANALYSIS_PROMPT_TYPE,
    title: SHARED_SHOT_ANALYSIS_TITLE,
}];

fn shared_shot_analysis_specs(
    current_workflow_mode: PromptWorkflowMode,
) -> Vec<SharedShotAnalysisSpec> {
    let mut specs = SHARED_SHOT_ANALYSIS_SPECS.to_vec();
    specs.sort_by_key(|spec| {
        if spec.workflow_mode == current_workflow_mode {
            0
        } else {
            1
        }
    });
    specs
}

async fn reusable_shared_shot_analysis_output(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    current_workflow_mode: PromptWorkflowMode,
    source_metadata: &Value,
) -> Result<Option<(SharedShotAnalysisSpec, String)>, AppError> {
    for spec in shared_shot_analysis_specs(current_workflow_mode) {
        if let Some(output) = service
            .reusable_shared_shot_analysis_output(
                project_id,
                chapter_id,
                spec.workflow_mode.as_str(),
                spec.prompt_type,
                source_metadata,
            )
            .await?
        {
            return Ok(Some((spec, output)));
        }
    }
    Ok(None)
}

struct PreparedSegmentPlan {
    segment_stage_outputs: Value,
    segment_trace: Option<VideoPromptGenerationTrace>,
    segment_json: Value,
    segment_plan: Vec<VideoSegmentPlan>,
    shot_analysis_draft: Option<NewChapterVideoPrompt>,
}

struct ReusableSegmentPlan {
    segment_json: Value,
    segment_plan: Vec<VideoSegmentPlan>,
    draft: NewChapterVideoPrompt,
}

const SHARED_SHOT_ANALYSIS_WORKFLOW_MODE: PromptWorkflowMode = PromptWorkflowMode::BoundaryFrames;
const SHARED_SHOT_ANALYSIS_PROMPT_TYPE: &str = "frame_shot_analysis";
const SHARED_SHOT_ANALYSIS_TITLE: &str = "视频镜头语言分析";
const SHARED_SHOT_ANALYSIS_PROGRESS: &str = "正在分析视频镜头语言";
const SHARED_SEGMENT_PLAN_WORKFLOW_MODE: PromptWorkflowMode = PromptWorkflowMode::BoundaryFrames;
const SHARED_SEGMENT_PLAN_PROMPT_TYPE: &str = "frame_segment_plan";
const SHARED_SEGMENT_PLAN_TITLE: &str = "视频片段规划";
const SHARED_SEGMENT_PLAN_CONTEXT: &str = "视频分段规划";
const SHARED_SEGMENT_PLAN_PROGRESS: &str = "正在规划视频片段";
const SHARED_SEGMENT_PLAN_CONTRACT_VERSION: &str = FRAME_SEGMENT_PLAN_CONTRACT_VERSION;

struct FramePromptGenerationSpec {
    workflow_mode: PromptWorkflowMode,
    shot_analysis_prompt_type: &'static str,
    shot_analysis_title: &'static str,
    shot_analysis_progress: &'static str,
    segment_prompt_type: &'static str,
    frame_prompt_type: &'static str,
    frame_source_prompt_type: Option<&'static str>,
    downstream_frame_prompt_types: &'static [&'static str],
    segment_context: &'static str,
    frame_check_progress: &'static str,
    reused_frame_label: &'static str,
    frame_generate_progress: &'static str,
    frame_regenerate_progress: &'static str,
    frame_prompt_name: &'static str,
    frame_saved_progress: &'static str,
    final_progress: &'static str,
    video_prompt_label: &'static str,
    video_prompt_name: &'static str,
    video_prompt_review_name: &'static str,
    read_video_inputs_progress: &'static str,
    missing_video_inputs_message: &'static str,
    incomplete_frame_label: &'static str,
    generate_video_progress: &'static str,
    layout: FramePromptLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FramePromptLayout {
    BoundaryFrames,
    InitialFirstFrame,
    PerSegmentStoryboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum VideoUnitType {
    Content,
    Transition,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct VideoUnitPlan {
    unit_number: i32,
    unit_type: VideoUnitType,
    title: String,
    duration_secs: i32,
    source_segment_number: Option<i32>,
    from_segment_number: Option<i32>,
    to_segment_number: Option<i32>,
    start_frame_number: Option<i32>,
    end_frame_number: Option<i32>,
    storyboard_number: Option<i32>,
    source_segments: Vec<VideoSegmentPlan>,
}

impl VideoUnitPlan {
    fn representative_segment(&self) -> Result<&VideoSegmentPlan, AppError> {
        self.source_segments.first().ok_or_else(|| {
            AppError::InvalidArgument(format!("视频单元 #{} 缺少源片段", self.unit_number))
        })
    }
}

impl FramePromptGenerationSpec {
    fn is_chat_video(&self) -> bool {
        matches!(self.workflow_mode, PromptWorkflowMode::ChatVideo)
    }

    fn is_first_frame_chain(&self) -> bool {
        matches!(self.layout, FramePromptLayout::InitialFirstFrame)
    }

    fn is_storyboard(&self) -> bool {
        matches!(self.layout, FramePromptLayout::PerSegmentStoryboard)
    }

    fn segment_storage_workflow_mode(&self) -> PromptWorkflowMode {
        SHARED_SEGMENT_PLAN_WORKFLOW_MODE
    }

    fn segment_storage_prompt_type(&self) -> &'static str {
        SHARED_SEGMENT_PLAN_PROMPT_TYPE
    }

    fn segment_storage_title(&self) -> &'static str {
        SHARED_SEGMENT_PLAN_TITLE
    }

    fn segment_storage_context(&self) -> &'static str {
        SHARED_SEGMENT_PLAN_CONTEXT
    }

    fn segment_storage_progress(&self) -> &'static str {
        SHARED_SEGMENT_PLAN_PROGRESS
    }

    fn segment_storage_contract_version(&self) -> &'static str {
        SHARED_SEGMENT_PLAN_CONTRACT_VERSION
    }

    fn frame_source_prompt_type(&self) -> &'static str {
        self.frame_source_prompt_type
            .unwrap_or(SHARED_SEGMENT_PLAN_PROMPT_TYPE)
    }
}

fn frame_prompt_generation_spec_for_workflow(
    workflow_mode: &str,
) -> Result<&'static FramePromptGenerationSpec, AppError> {
    match workflow_mode {
        "boundary_frames" => Ok(&BOUNDARY_FRAME_PROMPT_GENERATION),
        "first_frame_chain" => Ok(&FIRST_FRAME_PROMPT_GENERATION),
        "chat_video" => Ok(&CHAT_VIDEO_FRAME_PROMPT_GENERATION),
        "storyboard" => Ok(&STORYBOARD_PROMPT_GENERATION),
        other => Err(AppError::InvalidArgument(format!(
            "视频片段规划不支持 workflow_mode={other}"
        ))),
    }
}

const BOUNDARY_FRAME_DOWNSTREAM_PROMPT_TYPES: [&str; 1] = ["frame_prompt"];
const FIRST_FRAME_DOWNSTREAM_PROMPT_TYPES: [&str; 1] = ["first_frame_prompt"];
const CHAT_VIDEO_FRAME_DOWNSTREAM_PROMPT_TYPES: [&str; 1] = ["chat_video_frame_prompt"];
const STORYBOARD_DOWNSTREAM_PROMPT_TYPES: [&str; 1] = ["storyboard_prompt"];

const BOUNDARY_FRAME_PROMPT_GENERATION: FramePromptGenerationSpec = FramePromptGenerationSpec {
    workflow_mode: PromptWorkflowMode::BoundaryFrames,
    shot_analysis_prompt_type: SHARED_SHOT_ANALYSIS_PROMPT_TYPE,
    shot_analysis_title: SHARED_SHOT_ANALYSIS_TITLE,
    shot_analysis_progress: SHARED_SHOT_ANALYSIS_PROGRESS,
    segment_prompt_type: SHARED_SEGMENT_PLAN_PROMPT_TYPE,
    frame_prompt_type: "frame_prompt",
    frame_source_prompt_type: None,
    downstream_frame_prompt_types: &BOUNDARY_FRAME_DOWNSTREAM_PROMPT_TYPES,
    segment_context: SHARED_SEGMENT_PLAN_CONTEXT,
    frame_check_progress: "视频片段已规划，正在检查边界帧断点",
    reused_frame_label: "边界帧提示词",
    frame_generate_progress: "正在生成边界帧提示词",
    frame_regenerate_progress: "正在重新生成边界帧提示词",
    frame_prompt_name: "chapter_video_frame_prompts",
    frame_saved_progress: "边界帧提示词已保存，正在写入记录",
    final_progress: "边界帧提示词记录已更新",
    video_prompt_label: "视频提示词",
    video_prompt_name: "chapter_video_prompt",
    video_prompt_review_name: "chapter_video_prompt_review",
    read_video_inputs_progress: "正在读取边界帧提示词",
    missing_video_inputs_message: "请先生成当前章节的视频片段规划，再生成视频提示词",
    incomplete_frame_label: "边界帧提示词",
    generate_video_progress: "正在逐段生成视频提示词",
    layout: FramePromptLayout::BoundaryFrames,
};

const FIRST_FRAME_PROMPT_GENERATION: FramePromptGenerationSpec = FramePromptGenerationSpec {
    workflow_mode: PromptWorkflowMode::FirstFrameChain,
    shot_analysis_prompt_type: SHARED_SHOT_ANALYSIS_PROMPT_TYPE,
    shot_analysis_title: SHARED_SHOT_ANALYSIS_TITLE,
    shot_analysis_progress: SHARED_SHOT_ANALYSIS_PROGRESS,
    segment_prompt_type: SHARED_SEGMENT_PLAN_PROMPT_TYPE,
    frame_prompt_type: "first_frame_prompt",
    frame_source_prompt_type: Some(SHARED_SEGMENT_PLAN_PROMPT_TYPE),
    downstream_frame_prompt_types: &FIRST_FRAME_DOWNSTREAM_PROMPT_TYPES,
    segment_context: SHARED_SEGMENT_PLAN_CONTEXT,
    frame_check_progress: "视频片段已规划，正在检查初始首帧提示词",
    reused_frame_label: "初始首帧提示词",
    frame_generate_progress: "正在生成初始首帧提示词",
    frame_regenerate_progress: "正在重新生成初始首帧提示词",
    frame_prompt_name: "chapter_video_first_frame_prompts",
    frame_saved_progress: "初始首帧提示词已保存，正在写入记录",
    final_progress: "初始首帧提示词记录已更新",
    video_prompt_label: "首帧串联视频提示词",
    video_prompt_name: "chapter_video_first_frame_video_prompt",
    video_prompt_review_name: "chapter_video_first_frame_video_prompt_review",
    read_video_inputs_progress: "正在读取共享视频片段规划",
    missing_video_inputs_message: "请先生成当前章节的视频片段规划，再生成首帧串联视频提示词",
    incomplete_frame_label: "首帧提示词",
    generate_video_progress: "正在逐段生成首帧串联视频提示词",
    layout: FramePromptLayout::InitialFirstFrame,
};

const CHAT_VIDEO_FRAME_PROMPT_GENERATION: FramePromptGenerationSpec = FramePromptGenerationSpec {
    workflow_mode: PromptWorkflowMode::ChatVideo,
    shot_analysis_prompt_type: SHARED_SHOT_ANALYSIS_PROMPT_TYPE,
    shot_analysis_title: SHARED_SHOT_ANALYSIS_TITLE,
    shot_analysis_progress: SHARED_SHOT_ANALYSIS_PROGRESS,
    segment_prompt_type: SHARED_SEGMENT_PLAN_PROMPT_TYPE,
    frame_prompt_type: "chat_video_frame_prompt",
    frame_source_prompt_type: Some(SHARED_SEGMENT_PLAN_PROMPT_TYPE),
    downstream_frame_prompt_types: &CHAT_VIDEO_FRAME_DOWNSTREAM_PROMPT_TYPES,
    segment_context: SHARED_SEGMENT_PLAN_CONTEXT,
    frame_check_progress: "Chat 视频片段已规划，正在检查首尾帧断点",
    reused_frame_label: "Chat 首尾帧提示词",
    frame_generate_progress: "正在生成 Chat 首尾帧提示词",
    frame_regenerate_progress: "正在重新生成 Chat 首尾帧提示词",
    frame_prompt_name: "chapter_video_frame_prompts",
    frame_saved_progress: "Chat 首尾帧提示词已保存，正在写入记录",
    final_progress: "Chat 首尾帧提示词记录已更新",
    video_prompt_label: "Chat 视频提示词",
    video_prompt_name: "chapter_video_chat_prompt",
    video_prompt_review_name: "chapter_video_chat_prompt_review",
    read_video_inputs_progress: "正在读取共享视频片段规划",
    missing_video_inputs_message: "请先生成当前章节的视频片段规划，再生成 Chat 视频提示词",
    incomplete_frame_label: "Chat 首尾帧提示词",
    generate_video_progress: "正在逐段生成 Chat 视频提示词",
    layout: FramePromptLayout::BoundaryFrames,
};

const STORYBOARD_PROMPT_GENERATION: FramePromptGenerationSpec = FramePromptGenerationSpec {
    workflow_mode: PromptWorkflowMode::Storyboard,
    shot_analysis_prompt_type: SHARED_SHOT_ANALYSIS_PROMPT_TYPE,
    shot_analysis_title: SHARED_SHOT_ANALYSIS_TITLE,
    shot_analysis_progress: SHARED_SHOT_ANALYSIS_PROGRESS,
    segment_prompt_type: SHARED_SEGMENT_PLAN_PROMPT_TYPE,
    frame_prompt_type: "storyboard_prompt",
    frame_source_prompt_type: Some(SHARED_SEGMENT_PLAN_PROMPT_TYPE),
    downstream_frame_prompt_types: &STORYBOARD_DOWNSTREAM_PROMPT_TYPES,
    segment_context: SHARED_SEGMENT_PLAN_CONTEXT,
    frame_check_progress: "视频片段已规划，正在检查分镜提示词",
    reused_frame_label: "分镜提示词",
    frame_generate_progress: "正在生成分镜提示词",
    frame_regenerate_progress: "正在重新生成分镜提示词",
    frame_prompt_name: "chapter_video_storyboard_prompts",
    frame_saved_progress: "分镜提示词已保存，正在写入记录",
    final_progress: "分镜提示词记录已更新",
    video_prompt_label: "分镜视频提示词",
    video_prompt_name: "chapter_video_storyboard_video_prompt",
    video_prompt_review_name: "chapter_video_storyboard_video_prompt_review",
    read_video_inputs_progress: "正在读取共享视频片段规划和分镜提示词",
    missing_video_inputs_message: "请先生成当前章节的视频片段规划，再生成分镜视频提示词",
    incomplete_frame_label: "分镜提示词",
    generate_video_progress: "正在逐段生成分镜视频提示词",
    layout: FramePromptLayout::PerSegmentStoryboard,
};

fn expected_boundary_frame_count(segment_plan: &[VideoSegmentPlan]) -> usize {
    segment_plan.len() * 2
}

pub(in crate::video) fn expected_video_unit_count(segment_plan: &[VideoSegmentPlan]) -> usize {
    segment_plan.len().saturating_mul(2).saturating_sub(1)
}

fn video_units_for_segment_plan(
    segment_plan: &[VideoSegmentPlan],
    duration_secs: i32,
) -> Vec<VideoUnitPlan> {
    let mut units = Vec::with_capacity(expected_video_unit_count(segment_plan));
    for (index, segment) in segment_plan.iter().enumerate() {
        let segment_number = segment.segment_number;
        let content_unit_number = (index as i32 * 2) + 1;
        let start_frame_number = (index as i32 * 2) + 1;
        let end_frame_number = start_frame_number + 1;
        units.push(VideoUnitPlan {
            unit_number: content_unit_number,
            unit_type: VideoUnitType::Content,
            title: segment.title.clone(),
            duration_secs,
            source_segment_number: Some(segment_number),
            from_segment_number: None,
            to_segment_number: None,
            start_frame_number: Some(start_frame_number),
            end_frame_number: Some(end_frame_number),
            storyboard_number: Some(content_unit_number),
            source_segments: vec![segment.clone()],
        });
        if let Some(next_segment) = segment_plan.get(index + 1) {
            let transition_unit_number = content_unit_number + 1;
            units.push(VideoUnitPlan {
                unit_number: transition_unit_number,
                unit_type: VideoUnitType::Transition,
                title: format!("过渡：{} -> {}", segment.title, next_segment.title),
                duration_secs,
                source_segment_number: None,
                from_segment_number: Some(segment_number),
                to_segment_number: Some(next_segment.segment_number),
                start_frame_number: Some(end_frame_number),
                end_frame_number: Some(end_frame_number + 1),
                storyboard_number: Some(transition_unit_number),
                source_segments: vec![segment.clone(), next_segment.clone()],
            });
        }
    }
    units
}

fn boundary_frame_expected_segment_numbers(
    segment_plan: &[VideoSegmentPlan],
    frame_number: i32,
) -> Result<Vec<i32>, AppError> {
    if frame_number < 1 || frame_number as usize > expected_boundary_frame_count(segment_plan) {
        return Err(AppError::InvalidArgument(format!(
            "边界帧 frame_number {} 超出片段规划范围",
            frame_number
        )));
    }
    let segment_index = ((frame_number - 1) / 2) as usize;
    let segment = segment_plan
        .get(segment_index)
        .ok_or_else(|| AppError::InvalidArgument("边界帧缺少对应片段".to_string()))?;
    Ok(vec![segment.segment_number])
}

fn boundary_frame_role(
    segment_plan: &[VideoSegmentPlan],
    frame_number: i32,
) -> Result<&'static str, AppError> {
    if frame_number < 1 || frame_number as usize > expected_boundary_frame_count(segment_plan) {
        return Err(AppError::InvalidArgument(format!(
            "边界帧 frame_number {} 超出片段规划范围",
            frame_number
        )));
    }
    if frame_number % 2 == 1 {
        Ok("segment_start")
    } else {
        Ok("segment_end")
    }
}

pub(in crate::video) fn storyboard_expected_segment_numbers(
    segment_plan: &[VideoSegmentPlan],
    storyboard_number: i32,
) -> Result<Vec<i32>, AppError> {
    if storyboard_number < 1 || storyboard_number as usize > expected_video_unit_count(segment_plan)
    {
        return Err(AppError::InvalidArgument(format!(
            "分镜 frame_number {} 超出视频单元范围",
            storyboard_number
        )));
    }
    if storyboard_number % 2 == 1 {
        let segment = segment_plan
            .get(((storyboard_number - 1) / 2) as usize)
            .ok_or_else(|| AppError::InvalidArgument("分镜缺少对应片段".to_string()))?;
        Ok(vec![segment.segment_number])
    } else {
        let previous = segment_plan
            .get(((storyboard_number - 2) / 2) as usize)
            .ok_or_else(|| AppError::InvalidArgument("过渡分镜缺少前一片段".to_string()))?;
        let next = segment_plan
            .get((storyboard_number / 2) as usize)
            .ok_or_else(|| AppError::InvalidArgument("过渡分镜缺少后一片段".to_string()))?;
        Ok(vec![previous.segment_number, next.segment_number])
    }
}

fn boundary_frame_expected_duration(
    segment_plan: &[VideoSegmentPlan],
    segment_numbers: &[i32],
) -> Result<i32, AppError> {
    let mut duration = None;
    for segment_number in segment_numbers {
        let segment = segment_plan
            .iter()
            .find(|segment| segment.segment_number == *segment_number)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("边界帧引用了不存在的片段 {}", segment_number))
            })?;
        if let Some(expected) = duration {
            if expected != segment.duration_secs {
                return Err(AppError::InvalidArgument(format!(
                    "边界帧关联片段时长不一致：{}s 与 {}s",
                    expected, segment.duration_secs
                )));
            }
        } else {
            duration = Some(segment.duration_secs);
        }
    }
    duration.ok_or_else(|| AppError::InvalidArgument("边界帧 segment_numbers 不能为空".to_string()))
}

fn validate_single_boundary_frame_prompt(
    frame: &VideoFramePrompt,
    segment_plan: &[VideoSegmentPlan],
    allowed_reference_ids: &HashSet<i64>,
    required_first_reference_image_id: Option<i64>,
) -> Result<(), AppError> {
    let expected_segment_numbers =
        boundary_frame_expected_segment_numbers(segment_plan, frame.frame_number)?;
    if frame.segment_numbers != expected_segment_numbers {
        return Err(AppError::InvalidArgument(format!(
            "边界帧 #{} segment_numbers 必须为 {:?}，实际为 {:?}",
            frame.frame_number, expected_segment_numbers, frame.segment_numbers
        )));
    }
    let expected_duration = boundary_frame_expected_duration(segment_plan, &frame.segment_numbers)?;
    if frame.duration_secs != expected_duration {
        return Err(AppError::InvalidArgument(format!(
            "边界帧 #{} duration_secs 必须为 {}，实际为 {}",
            frame.frame_number, expected_duration, frame.duration_secs
        )));
    }
    for segment_number in &frame.segment_numbers {
        let segment = segment_plan
            .iter()
            .find(|segment| segment.segment_number == *segment_number)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "边界帧 #{} 引用了不存在的片段 {}",
                    frame.frame_number, segment_number
                ))
            })?;
        if frame.aspect_ratio != segment.aspect_ratio {
            return Err(AppError::InvalidArgument(format!(
                "边界帧 #{} aspect_ratio 必须为 {}，实际为 {}",
                frame.frame_number, segment.aspect_ratio, frame.aspect_ratio
            )));
        }
    }
    validate_direct_prompt_text(
        &frame.frame_prompt,
        &format!("边界帧 #{} frame_prompt", frame.frame_number),
    )?;
    validate_direct_prompt_text(
        &frame.negative_prompt,
        &format!("边界帧 #{} negative_prompt", frame.frame_number),
    )?;
    validate_reference_label_text(
        &frame.frame_prompt,
        &format!("边界帧 #{} frame_prompt", frame.frame_number),
        frame.reference_image_ids.len(),
    )?;
    validate_reference_label_text(
        &frame.negative_prompt,
        &format!("边界帧 #{} negative_prompt", frame.frame_number),
        frame.reference_image_ids.len(),
    )?;
    ensure_video_reference_ids_known(
        &frame.reference_image_ids,
        allowed_reference_ids,
        &format!("视频边界帧 #{}", frame.frame_number),
    )?;
    if let Some(required_id) = required_first_reference_image_id
        && frame.reference_image_ids.first().copied() != Some(required_id)
    {
        return Err(AppError::InvalidArgument(format!(
            "边界帧 #{} reference_image_ids[0] 必须是上一帧实际图片 ID {}，这样 frame_prompt 中的“参考图 1”才对应上一帧；实际为 {:?}",
            frame.frame_number,
            required_id,
            frame.reference_image_ids.first()
        )));
    }
    Ok(())
}

fn validate_partial_boundary_frame_prompts(
    frames: &[VideoFramePrompt],
    segment_plan: &[VideoSegmentPlan],
    allowed_reference_ids: &HashSet<i64>,
    image_id_by_frame_number: &HashMap<i32, i64>,
) -> Result<(), AppError> {
    for (index, frame) in frames.iter().enumerate() {
        let expected_frame_number = (index + 1) as i32;
        if frame.frame_number != expected_frame_number {
            return Err(AppError::InvalidArgument(format!(
                "边界帧提示词必须是从 1 开始的连续前缀，位置 {} 应为 #{}，实际为 #{}",
                index + 1,
                expected_frame_number,
                frame.frame_number
            )));
        }
        validate_single_boundary_frame_prompt(
            frame,
            segment_plan,
            allowed_reference_ids,
            frame
                .frame_number
                .checked_sub(1)
                .and_then(|number| image_id_by_frame_number.get(&number).copied()),
        )?;
    }
    Ok(())
}

pub(in crate::video) fn boundary_frame_prompt_rows_prefix(
    rows: Vec<chapter_video_prompt::Model>,
    segment_plan: &[VideoSegmentPlan],
    allowed_reference_ids: &HashSet<i64>,
    image_id_by_frame_number: &HashMap<i32, i64>,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let mut prefix = Vec::new();
    for row in rows {
        let expected = (prefix.len() + 1) as i32;
        if row.shot_number == expected {
            let frame = match frame_prompts_from_rows(std::slice::from_ref(&row)) {
                Ok(mut frames) if frames.len() == 1 => frames.remove(0),
                Ok(frames) => {
                    tracing::info!(
                        target: "a4n::video_prompt",
                        prompt_id = row.id,
                        frame_count = frames.len(),
                        "已保存边界帧提示词不可复用，将从该帧开始重新生成"
                    );
                    break;
                }
                Err(err) => {
                    tracing::info!(
                        target: "a4n::video_prompt",
                        prompt_id = row.id,
                        error = %err,
                        "已保存边界帧提示词不可复用，将从该帧开始重新生成"
                    );
                    break;
                }
            };
            if let Err(err) = validate_single_boundary_frame_prompt(
                &frame,
                segment_plan,
                allowed_reference_ids,
                frame
                    .frame_number
                    .checked_sub(1)
                    .and_then(|number| image_id_by_frame_number.get(&number).copied()),
            ) {
                tracing::info!(
                    target: "a4n::video_prompt",
                    prompt_id = row.id,
                    frame_number = frame.frame_number,
                    error = %err,
                    "已保存边界帧提示词校验失败，将从该帧开始重新生成"
                );
                break;
            }
            prefix.push(row);
        } else if row.shot_number > expected {
            break;
        }
    }
    Ok(prefix)
}

async fn sync_reusable_boundary_frame_previous_image_refs(
    service: &VideoService,
    rows: Vec<chapter_video_prompt::Model>,
    image_id_by_frame_number: &HashMap<i32, i64>,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let mut synced = Vec::with_capacity(rows.len());
    for row in rows {
        synced.push(
            sync_boundary_frame_previous_image_ref(service, row, image_id_by_frame_number).await?,
        );
    }
    Ok(synced)
}

async fn sync_boundary_frame_previous_image_ref(
    service: &VideoService,
    row: chapter_video_prompt::Model,
    image_id_by_frame_number: &HashMap<i32, i64>,
) -> Result<chapter_video_prompt::Model, AppError> {
    if row.shot_number <= 1 {
        return Ok(row);
    }
    let Some(latest_previous_image_id) = image_id_by_frame_number
        .get(&(row.shot_number - 1))
        .copied()
    else {
        return Ok(row);
    };
    let refs = prompt_reference_ids(row.reference_image_ids_json.as_ref())?;
    if refs.is_empty() || refs.contains(&latest_previous_image_id) {
        return Ok(row);
    }
    let Some(last_ref_id) = refs.last().copied() else {
        return Ok(row);
    };
    let last_ref_exists = generated_image::Entity::find_by_id(last_ref_id)
        .one(service.db.as_ref())
        .await
        .map_err(AppError::Database)?
        .is_some();
    if last_ref_exists {
        return Ok(row);
    }
    let updated_refs = refs
        .into_iter()
        .map(|id| {
            if id == last_ref_id {
                latest_previous_image_id
            } else {
                id
            }
        })
        .fold(Vec::<i64>::new(), |mut acc, id| {
            if !acc.contains(&id) {
                acc.push(id);
            }
            acc
        });
    let mut metadata_json = row.metadata_json.clone();
    if let Some(metadata) = metadata_json.as_mut()
        && let Some(boundary_frame) = metadata.get_mut("boundary_frame")
        && let Some(object) = boundary_frame.as_object_mut()
    {
        object.insert(
            "reference_image_ids".to_string(),
            json!(updated_refs.clone()),
        );
    }
    let mut active: chapter_video_prompt::ActiveModel = row.clone().into();
    active.reference_image_ids_json = Set(Some(json!(updated_refs)));
    active.metadata_json = Set(metadata_json);
    active.updated_at = Set(chrono::Utc::now().naive_utc());
    active
        .update(service.db.as_ref())
        .await
        .map_err(AppError::Database)
}

fn prompt_reference_ids(value: Option<&Value>) -> Result<Vec<i64>, AppError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let items = value.as_array().ok_or_else(|| {
        AppError::InvalidArgument("reference_image_ids_json 必须是数组".to_string())
    })?;
    let mut ids = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let id = item.as_i64().ok_or_else(|| {
            AppError::InvalidArgument(format!("reference_image_ids_json[{index}] 必须是整数"))
        })?;
        if id <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "reference_image_ids_json[{index}] 必须为正整数"
            )));
        }
        ids.push(id);
    }
    Ok(ids)
}

pub(in crate::video) fn reusable_frame_prefix_len_with_previous_images(
    frames: &[VideoFramePrompt],
    image_id_by_frame_number: &HashMap<i32, i64>,
) -> usize {
    for (index, frame) in frames.iter().enumerate() {
        if frame.frame_number <= 1 || image_id_by_frame_number.contains_key(&frame.frame_number) {
            continue;
        }
        let Some(previous_image_id) = image_id_by_frame_number.get(&(frame.frame_number - 1))
        else {
            continue;
        };
        if !frame.reference_image_ids.contains(previous_image_id) {
            return index;
        }
    }
    frames.len()
}

async fn ensure_boundary_frame_checkpoint_can_sync(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
    kept_prompt_ids: &BTreeSet<i64>,
) -> Result<(), AppError> {
    let rows = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode.as_str()))
        .filter(chapter_video_prompt::Column::PromptType.eq(prompt_type))
        .all(service.db.as_ref())
        .await
        .map_err(AppError::Database)?;
    for row in rows {
        if row.status == "running" {
            return Err(AppError::InvalidArgument(format!(
                "当前章节已有边界帧提示词 #{} 正在运行，不能继续生成边界帧提示词",
                row.shot_number
            )));
        }
        if !kept_prompt_ids.contains(&row.id)
            && !service.frame_prompt_has_generated_image(&row).await?
        {
            service.ensure_prompt_stage_mutable(&row, "删除").await?;
        }
    }
    Ok(())
}
