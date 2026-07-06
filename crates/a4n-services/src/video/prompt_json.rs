use a4n_core::error::AppError;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

use crate::chapter::source_spans::SourceSpan;
use crate::prompt_language::PromptOutputLanguage;

mod chat_prompts;
mod derived_fields;
mod validation;

pub(super) use self::chat_prompts::{
    ChatVideoPromptShot, parse_chat_video_prompt_shots,
    validate_chat_video_prompt_shot_against_segment,
};
use self::derived_fields::video_prompt_shot_from_input;
pub(in crate::video) use self::validation::direct_prompt_internal_reference_term;
pub use self::validation::{
    validate_direct_prompt_text, validate_reference_label_text,
    video_motion_prompt_has_temporal_storyboard,
    video_prompt_missing_required_section_for_language, video_prompt_stale_section_for_language,
    video_prompt_upload_status_term,
};
use self::validation::{
    validate_frame_prompt, validate_model_shot, validate_segment_plan, validate_shot,
};

pub(in crate::video) const VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS: &[&str] = &[
    "speaker",
    "line",
    "delivery",
    "visible_action",
    "listener_reaction",
    "timing",
];

pub(in crate::video) fn sanitize_prompt_asset_text(value: Option<&Value>) -> Option<String> {
    let text = value.and_then(Value::as_str)?.trim();
    if text.is_empty() {
        return None;
    }
    let text = remove_internal_asset_identifiers(text);
    let text = text.trim();
    if text.is_empty() || direct_prompt_internal_reference_term(text).is_some() {
        None
    } else {
        Some(text.to_string())
    }
}

fn remove_internal_asset_identifiers(text: &str) -> String {
    remove_identifier_tokens_with_marker(text, "manual_library_asset_")
}

fn remove_identifier_tokens_with_marker(text: &str, marker: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(position) = rest.find(marker) {
        output.push_str(&rest[..position]);
        let after_marker = &rest[position + marker.len()..];
        let consumed = after_marker
            .char_indices()
            .take_while(|(_, ch)| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
            .map(|(index, ch)| index + ch.len_utf8())
            .last()
            .unwrap_or(0);
        rest = &after_marker[consumed..];
    }
    output.push_str(rest);
    output
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct VideoPromptShot {
    pub shot_number: i32,
    pub title: String,
    pub scene_summary: String,
    pub self_contained_context: Value,
    pub start_state: Value,
    pub end_state: Value,
    pub transition_in: Value,
    pub visual_prompt: String,
    pub motion_prompt: String,
    pub negative_prompt: String,
    pub blocking_plan: Value,
    pub lighting_plan: Value,
    pub emotion_action_beats: Value,
    pub dialogue_beats: Value,
    pub cinematography_plan: Value,
    pub temporal_beats: Value,
    pub reference_binding: Value,
    pub continuity_check: Value,
    pub duration_secs: i32,
    pub aspect_ratio: String,
    pub reference_image_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct VideoSegmentPlan {
    pub segment_number: i32,
    pub analysis_shot_id: i32,
    pub segment_index_in_shot: i32,
    pub shot_segment_count: i32,
    pub covered_beat_ids: Vec<i32>,
    pub title: String,
    pub scene_summary: String,
    pub blocking_summary: String,
    pub spatial_anchors: Value,
    pub emotion_action_goals: Value,
    pub video_intent: Value,
    pub emotional_arc: Value,
    pub performance_plan: Value,
    pub material_coverage: Value,
    pub source_span_ids: Vec<String>,
    pub duration_secs: i32,
    pub aspect_ratio: String,
    pub reference_image_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct VideoShotAnalysis {
    pub cinematic_shots: Vec<VideoCinematicShot>,
    pub planning_quality: VideoPlanningQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct VideoCinematicShot {
    pub shot_id: i32,
    pub title: String,
    pub source_span_ids: Vec<String>,
    pub source_span_range: VideoSourceSpanRange,
    pub key_source_span_ids: Vec<String>,
    pub scene_intent: String,
    pub visible_actions: Value,
    pub dialogue_or_speech_intent: Value,
    pub blocking: Value,
    pub camera_intent: Value,
    pub transition_in: String,
    pub transition_out: String,
    pub start_state: Value,
    pub end_state: Value,
    pub action_beats: Vec<VideoActionBeat>,
    pub estimated_natural_seconds: f64,
    pub segment_duration_secs: i32,
    pub required_segment_count: i32,
    pub action_load: String,
    pub continuity_handoff: Value,
    pub risk_flags: Vec<VideoPlanningRisk>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct VideoSourceSpanRange {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct VideoActionBeat {
    pub beat_id: i32,
    pub beat_type: String,
    pub natural_seconds: f64,
    pub required: bool,
    pub visible_action: String,
    pub start_state: Option<Value>,
    pub end_state: Option<Value>,
    pub source_span_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct VideoPlanningQuality {
    pub total_estimated_seconds: f64,
    pub total_required_segments: i32,
    pub risk_flags: Vec<VideoPlanningRisk>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct VideoPlanningRisk {
    pub level: String,
    pub code: String,
    pub shot_id: Option<i32>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct VideoFramePrompt {
    pub frame_number: i32,
    pub title: String,
    pub scene_summary: String,
    #[serde(default)]
    pub storyboard_panels: Vec<StoryboardPanel>,
    pub frame_prompt: String,
    pub blocking_state: Value,
    pub lighting_state: Value,
    pub negative_prompt: String,
    pub duration_secs: i32,
    pub aspect_ratio: String,
    pub reference_image_ids: Vec<i64>,
    pub segment_numbers: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::video) struct StoryboardPanel {
    pub panel_index: i32,
    pub moment: String,
    pub visual_goal: String,
    pub composition: String,
    pub continuity_note: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VideoPromptShotInput {
    pub title: String,
    pub scene_summary: String,
    pub visual_prompt: String,
    pub motion_prompt: String,
    pub negative_prompt: String,
    pub dialogue_cues: Value,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VideoSegmentPlanInput {
    pub analysis_shot_id: i32,
    pub covered_beat_ids: Vec<i32>,
    pub title: String,
    pub scene_summary: String,
    pub blocking_summary: String,
    pub spatial_anchors: Value,
    pub emotion_action_goals: Value,
    pub video_intent: Value,
    pub emotional_arc: Value,
    pub performance_plan: Value,
    pub material_coverage: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VideoFramePromptInput {
    pub title: String,
    pub scene_summary: String,
    #[serde(default)]
    pub storyboard_panels: Vec<StoryboardPanel>,
    pub frame_prompt: String,
    pub blocking_state: Value,
    pub lighting_state: Value,
    pub negative_prompt: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VideoBeatExtractionResponse {
    beats: Vec<VideoActionBeatInput>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VideoShotGroupingResponse {
    cinematic_shots: Vec<VideoCinematicShotGroupingInput>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredVideoShotAnalysis {
    cinematic_shots: Vec<VideoCinematicShot>,
    #[serde(rename = "planning_quality")]
    _planning_quality: VideoPlanningQuality,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VideoCinematicShotGroupingInput {
    pub beat_ids: Vec<i32>,
    pub title: String,
    pub scene_intent: String,
    pub visible_actions: Value,
    pub dialogue_or_speech_intent: Value,
    pub blocking: Value,
    pub camera_intent: Value,
    pub transition_in: String,
    pub transition_out: String,
    pub start_state: Value,
    pub end_state: Value,
    pub action_load: String,
    pub continuity_handoff: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VideoActionBeatInput {
    pub natural_seconds: f64,
    pub required: bool,
    pub visible_action: String,
    pub source_span_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VideoPromptResponse {
    shots: Vec<VideoPromptShotInput>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VideoSegmentPlanResponse {
    segments: Vec<VideoSegmentPlanInput>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredVideoSegmentPlanResponse {
    segments: Vec<VideoSegmentPlan>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VideoFramePromptResponse {
    frames: Vec<VideoFramePromptInput>,
}

pub(super) struct VideoFramePromptDefaults<'a> {
    pub frame_number: i32,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub segment_numbers: Vec<i32>,
    pub reference_image_ids: Vec<i64>,
}

struct VideoSegmentPlanDefaults<'a> {
    shot_analysis: &'a VideoShotAnalysis,
    duration_secs: i32,
    aspect_ratio: &'a str,
}

fn parse_strict_json_response<T>(raw: &str, context: &str) -> Result<T, AppError>
where
    T: DeserializeOwned,
{
    let trimmed = raw.trim();
    let contract_hint = strict_json_contract_hint(trimmed);
    serde_json::from_str(trimmed).map_err(|err| {
        let parse_hint = strict_json_parse_error_hint(&err);
        AppError::InvalidArgument(format!(
            "{context} 响应必须是严格 JSON 对象{contract_hint}{parse_hint}: {err}; 输出预览：{}",
            json_response_preview(trimmed)
        ))
    })
}

fn strict_json_parse_error_hint(err: &serde_json::Error) -> &'static str {
    if err.to_string().contains("duplicate field") {
        return "；检测到 JSON 对象中重复字段，严格 JSON 响应中每个字段名只能出现一次；请让模型把补充内容合并到原字段内部，不要重复输出同名 key";
    }
    if matches!(err.classify(), serde_json::error::Category::Eof) {
        return "；响应在 JSON 闭合前结束，通常是模型输出被截断、连接提前结束或 max_tokens 不足";
    }
    ""
}

fn strict_json_contract_hint(text: &str) -> &'static str {
    let lower = text.trim_start().to_ascii_lowercase();
    if lower.starts_with("```") {
        return "；检测到 Markdown 代码围栏，严格 JSON 响应必须直接以 JSON 对象开始，不能用 ``` 或 ```json 包裹";
    }
    if lower.starts_with("<antml thinking>")
        || lower.starts_with("<thinking>")
        || lower.starts_with("<think>")
    {
        return "；检测到模型把 thinking/reasoning 标签写入了正文 content，说明当前 LLM/代理没有遵守严格 JSON response_format；请关闭该模型配置的思考输出、降低/禁用 reasoning，或改用支持 JSON mode 的 LLM 配置";
    }
    ""
}

fn json_response_preview(trimmed: &str) -> String {
    if trimmed.is_empty() {
        return "<空响应>".to_string();
    }

    let mut chars = trimmed.chars();
    let mut preview = chars.by_ref().take(240).collect::<String>();
    if chars.next().is_some() {
        preview.push_str("...");
    }
    preview
        .replace('\r', "\\r")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

pub(super) fn parse_video_prompt_shots(
    raw: &str,
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<Vec<VideoPromptShot>, AppError> {
    let response: VideoPromptResponse = parse_strict_json_response(raw, "视频提示词")?;
    if response.shots.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词响应必须包含至少一个 shots 条目".to_string(),
        ));
    }

    let mut seen_numbers = HashSet::new();
    let ordered_segment_numbers = ordered_segment_numbers(segment_by_number);
    let shot_count = response.shots.len();
    let mut shots = Vec::with_capacity(response.shots.len());
    for (index, input) in response.shots.into_iter().enumerate() {
        let shot_number = derived_prompt_shot_number(
            index,
            shot_count,
            &ordered_segment_numbers,
            "视频提示词 shot_number",
        )?;
        let segment = segment_by_number.get(&shot_number).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频提示词 shot_number {} 必须对应已有片段编号",
                shot_number
            ))
        })?;
        let shot = video_prompt_shot_from_input(input, shot_number, segment);
        let shot =
            validate_model_video_prompt_shot_against_segment(shot, segment, prompt_output_language)
                .map_err(|err| match err {
                    AppError::InvalidArgument(message) => AppError::InvalidArgument(format!(
                        "视频提示词 shot_number {} 无效：{}",
                        shot_number, message
                    )),
                    other => other,
                })?;
        if !seen_numbers.insert(shot.shot_number) {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词 shot_number {} 重复",
                shot.shot_number
            )));
        }
        shots.push(shot);
    }
    Ok(shots)
}

pub(super) fn parse_video_segment_plan_response(
    raw: &str,
    source_spans: &[SourceSpan],
    shot_analysis: &VideoShotAnalysis,
    duration_secs: i32,
    default_aspect_ratio: &str,
) -> Result<Vec<VideoSegmentPlan>, AppError> {
    parse_video_segment_plan_inner(
        raw,
        source_spans,
        VideoSegmentPlanDefaults {
            shot_analysis,
            duration_secs,
            aspect_ratio: default_aspect_ratio,
        },
    )
}

pub(super) fn parse_video_segment_plan_input_values(
    raw: &str,
    context: &str,
) -> Result<Vec<Value>, AppError> {
    let response: VideoSegmentPlanResponse = parse_strict_json_response(raw, context)?;
    if response.segments.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{context} 响应必须包含至少一个 segments 条目"
        )));
    }
    response
        .segments
        .into_iter()
        .map(|segment| {
            serde_json::to_value(segment).map_err(|err| {
                AppError::InvalidArgument(format!("{context} segments 条目序列化失败: {err}"))
            })
        })
        .collect()
}

pub(super) fn parse_stored_video_segment_plan(
    raw: &str,
    source_spans: &[SourceSpan],
) -> Result<Vec<VideoSegmentPlan>, AppError> {
    let response: StoredVideoSegmentPlanResponse = serde_json::from_str(raw.trim())?;
    if response.segments.is_empty() {
        return Err(AppError::InvalidArgument(
            "已存视频分段规划必须包含至少一个 segments 条目".to_string(),
        ));
    }

    let mut seen_numbers = HashSet::new();
    let mut segments = Vec::with_capacity(response.segments.len());
    for segment in response.segments {
        let segment_number = segment.segment_number;
        if !segment.reference_image_ids.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "已存视频分段规划 segment_number {segment_number} 包含已废弃的 reference_image_ids；片段规划不再绑定具体素材，请删除旧规划并重新生成"
            )));
        }
        let source_span_ids = validate_source_span_ids(
            segment.source_span_ids.clone(),
            source_spans,
            &format!("已存视频分段规划 segment_number {}", segment_number),
        )?;
        let segment = VideoSegmentPlan {
            source_span_ids,
            ..segment
        };
        let segment = validate_segment_plan(segment)?;
        if !seen_numbers.insert(segment.segment_number) {
            return Err(AppError::InvalidArgument(format!(
                "已存视频分段规划 segment_number {} 重复",
                segment.segment_number
            )));
        }
        segments.push(segment);
    }
    validate_continuous_ids(
        segments.iter().map(|segment| segment.segment_number),
        "已存视频分段规划 segment_number",
    )?;
    Ok(segments)
}

pub(super) fn parse_stored_video_segment_plan_for_reuse(
    raw: &str,
    source_spans: &[SourceSpan],
    expected_aspect_ratio: &str,
) -> Result<Vec<VideoSegmentPlan>, AppError> {
    let response: StoredVideoSegmentPlanResponse = serde_json::from_str(raw.trim())?;
    if response.segments.is_empty() {
        return Err(AppError::InvalidArgument(
            "已存视频分段规划必须包含至少一个 segments 条目".to_string(),
        ));
    }

    let mut seen_numbers = HashSet::new();
    let mut segments = Vec::with_capacity(response.segments.len());
    for segment in response.segments {
        let segment_number = segment.segment_number;
        if segment.aspect_ratio != expected_aspect_ratio {
            return Err(AppError::InvalidArgument(format!(
                "已存视频分段规划 segment_number {segment_number} aspect_ratio 必须等于当前请求 {expected_aspect_ratio}，实际为 {}；请删除旧规划并重新生成",
                segment.aspect_ratio
            )));
        }
        if !segment.reference_image_ids.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "已存视频分段规划 segment_number {segment_number} 包含已废弃的 reference_image_ids；片段规划不再绑定具体素材，请删除旧规划并重新生成"
            )));
        }
        let source_span_ids = validate_source_span_ids(
            segment.source_span_ids.clone(),
            source_spans,
            &format!("已存视频分段规划 segment_number {}", segment_number),
        )?;
        let segment = VideoSegmentPlan {
            source_span_ids,
            ..segment
        };
        let segment = validate_segment_plan(segment)?;
        if !seen_numbers.insert(segment.segment_number) {
            return Err(AppError::InvalidArgument(format!(
                "已存视频分段规划 segment_number {} 重复",
                segment.segment_number
            )));
        }
        segments.push(segment);
    }
    validate_continuous_ids(
        segments.iter().map(|segment| segment.segment_number),
        "已存视频分段规划 segment_number",
    )?;
    Ok(segments)
}

fn parse_video_segment_plan_inner(
    raw: &str,
    source_spans: &[SourceSpan],
    defaults: VideoSegmentPlanDefaults<'_>,
) -> Result<Vec<VideoSegmentPlan>, AppError> {
    let response: VideoSegmentPlanResponse = serde_json::from_str(raw.trim())?;
    if response.segments.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频分段规划响应必须包含至少一个 segments 条目".to_string(),
        ));
    }
    let segment_count_by_shot =
        response
            .segments
            .iter()
            .fold(HashMap::<i32, i32>::new(), |mut counts, segment| {
                *counts.entry(segment.analysis_shot_id).or_default() += 1;
                counts
            });
    let mut next_index_by_shot = HashMap::<i32, i32>::new();
    let mut seen_numbers = HashSet::new();
    let mut segments = Vec::with_capacity(response.segments.len());
    for (index, segment) in response.segments.into_iter().enumerate() {
        let segment_number = (index + 1) as i32;
        let shot_segment_count = segment_count_by_shot
            .get(&segment.analysis_shot_id)
            .copied()
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频分段规划 segment_number {} 无法推导 shot_segment_count",
                    segment_number
                ))
            })?;
        let derived_index = next_index_by_shot
            .entry(segment.analysis_shot_id)
            .and_modify(|value| *value += 1)
            .or_insert(1);
        let segment_index_in_shot = *derived_index;
        let source_span_ids = derive_segment_source_span_ids(
            defaults.shot_analysis,
            segment.analysis_shot_id,
            &segment.covered_beat_ids,
            segment_number,
        )?;
        let segment = VideoSegmentPlan {
            segment_number,
            analysis_shot_id: segment.analysis_shot_id,
            segment_index_in_shot,
            shot_segment_count,
            covered_beat_ids: segment.covered_beat_ids,
            title: segment.title,
            scene_summary: segment.scene_summary,
            blocking_summary: segment.blocking_summary,
            spatial_anchors: segment.spatial_anchors,
            emotion_action_goals: segment.emotion_action_goals,
            video_intent: segment.video_intent,
            emotional_arc: segment.emotional_arc,
            performance_plan: segment.performance_plan,
            material_coverage: segment.material_coverage,
            source_span_ids: validate_source_span_ids(
                source_span_ids,
                source_spans,
                &format!("视频分段规划 segment_number {}", segment_number),
            )?,
            duration_secs: defaults.duration_secs,
            aspect_ratio: defaults.aspect_ratio.to_string(),
            reference_image_ids: Vec::new(),
        };
        let segment = validate_segment_plan(segment)?;
        if !seen_numbers.insert(segment.segment_number) {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划 segment_number {} 重复",
                segment.segment_number
            )));
        }
        segments.push(segment);
    }
    Ok(segments)
}

pub(super) fn parse_video_beat_extraction_response(
    raw: &str,
    source_spans: &[SourceSpan],
    first_beat_id: i32,
    context: &str,
) -> Result<Vec<VideoActionBeat>, AppError> {
    if first_beat_id < 1 {
        return Err(AppError::InvalidArgument(format!(
            "{context} first_beat_id 必须大于 0"
        )));
    }
    let response: VideoBeatExtractionResponse = parse_strict_json_response(raw, context)?;
    if response.beats.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{context} 响应必须包含至少一个 beats 条目"
        )));
    }
    response
        .beats
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            let beat_id = first_beat_id + index as i32;
            let visible_action =
                required_non_empty_text(item.visible_action, &format!("{context} visible_action"))?;
            if !item.natural_seconds.is_finite() || item.natural_seconds <= 0.0 {
                return Err(AppError::InvalidArgument(format!(
                    "{context} beat_id {} 的 natural_seconds 必须大于 0",
                    beat_id
                )));
            }
            let beat_type = infer_action_beat_type(&visible_action).to_string();
            Ok(VideoActionBeat {
                beat_id,
                beat_type,
                natural_seconds: item.natural_seconds,
                required: item.required,
                visible_action,
                start_state: None,
                end_state: None,
                source_span_ids: validate_source_span_ids(
                    item.source_span_ids,
                    source_spans,
                    &format!("{context} beat_id {}", beat_id),
                )?,
            })
        })
        .collect()
}

pub(super) fn parse_video_shot_grouping_response(
    raw: &str,
    source_spans: &[SourceSpan],
    expected_duration_secs: i32,
    extracted_beats: &[VideoActionBeat],
) -> Result<VideoShotAnalysis, AppError> {
    if extracted_beats.is_empty() {
        return Err(AppError::InvalidArgument(
            "镜头分组必须基于至少一个已抽取动作节拍".to_string(),
        ));
    }
    validate_continuous_ids(
        extracted_beats.iter().map(|beat| beat.beat_id),
        "动作节拍 beat_id",
    )?;
    let beat_by_id = extracted_beats
        .iter()
        .map(|beat| (beat.beat_id, beat))
        .collect::<HashMap<_, _>>();
    let response: VideoShotGroupingResponse = parse_strict_json_response(raw, "镜头分组")?;
    if response.cinematic_shots.is_empty() {
        return Err(AppError::InvalidArgument(
            "镜头分组响应必须包含至少一个 cinematic_shots 条目".to_string(),
        ));
    }
    let mut shots = Vec::with_capacity(response.cinematic_shots.len());
    let mut seen_ids = HashSet::new();
    let mut next_global_beat_id = 1;
    for (shot_index, input) in response.cinematic_shots.into_iter().enumerate() {
        let shot_id = (shot_index + 1) as i32;
        let context = format!("镜头分组 shot_id {}", shot_id);
        if !seen_ids.insert(shot_id) {
            return Err(AppError::InvalidArgument(format!(
                "镜头分组 shot_id {} 重复",
                shot_id
            )));
        }
        if input.beat_ids.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "{context} beat_ids 不能为空"
            )));
        }
        let mut action_beats = Vec::with_capacity(input.beat_ids.len());
        for (local_index, global_beat_id) in input.beat_ids.into_iter().enumerate() {
            if global_beat_id != next_global_beat_id {
                return Err(AppError::InvalidArgument(format!(
                    "{context} beat_ids 必须按动作节拍顺序连续覆盖，不得遗漏、重复或倒序；期望 {}，实际 {}",
                    next_global_beat_id, global_beat_id
                )));
            }
            let extracted = beat_by_id.get(&global_beat_id).ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "{context} beat_ids 包含不存在的动作节拍 ID：{}",
                    global_beat_id
                ))
            })?;
            let local_beat_id = (local_index + 1) as i32;
            action_beats.push(VideoActionBeat {
                beat_id: local_beat_id,
                beat_type: extracted.beat_type.clone(),
                natural_seconds: extracted.natural_seconds,
                required: extracted.required,
                visible_action: extracted.visible_action.clone(),
                start_state: extracted.start_state.clone(),
                end_state: extracted.end_state.clone(),
                source_span_ids: validate_source_span_ids(
                    extracted.source_span_ids.clone(),
                    source_spans,
                    &format!("{context} action_beats beat_id {}", local_beat_id),
                )?,
            });
            next_global_beat_id += 1;
        }
        let shot_source_span_ids = action_beat_source_span_ids(&action_beats);
        let shot_source_span_ids =
            validate_source_span_ids(shot_source_span_ids, source_spans, &context)?;
        let span_range = derive_source_span_range(&shot_source_span_ids, source_spans, &context)?;
        let key_source_span_ids = shot_source_span_ids.clone();
        let estimated_natural_seconds = action_beats
            .iter()
            .map(|beat| beat.natural_seconds)
            .sum::<f64>();
        let required_segment_count =
            required_segment_count(estimated_natural_seconds, expected_duration_secs)?;
        let risk_flags = shot_risk_flags(
            shot_id,
            &span_range,
            &action_beats,
            estimated_natural_seconds,
            required_segment_count,
            expected_duration_secs,
            source_spans,
        )?;
        let shot = VideoCinematicShot {
            shot_id,
            title: required_non_empty_text(input.title, "镜头语言分析 title")?,
            source_span_ids: shot_source_span_ids,
            source_span_range: span_range,
            key_source_span_ids,
            scene_intent: required_non_empty_text(input.scene_intent, "镜头语言分析 scene_intent")?,
            visible_actions: required_non_empty_value(
                input.visible_actions,
                "镜头语言分析 visible_actions",
            )?,
            dialogue_or_speech_intent: input.dialogue_or_speech_intent,
            blocking: required_non_empty_value(input.blocking, "镜头语言分析 blocking")?,
            camera_intent: required_non_empty_value(
                input.camera_intent,
                "镜头语言分析 camera_intent",
            )?,
            transition_in: required_non_empty_text(
                input.transition_in,
                "镜头语言分析 transition_in",
            )?,
            transition_out: required_non_empty_text(
                input.transition_out,
                "镜头语言分析 transition_out",
            )?,
            start_state: required_non_empty_value(input.start_state, "镜头语言分析 start_state")?,
            end_state: required_non_empty_value(input.end_state, "镜头语言分析 end_state")?,
            action_beats,
            estimated_natural_seconds,
            segment_duration_secs: expected_duration_secs,
            required_segment_count,
            action_load: required_non_empty_text(input.action_load, "镜头语言分析 action_load")?,
            continuity_handoff: required_non_empty_value(
                input.continuity_handoff,
                "镜头语言分析 continuity_handoff",
            )?,
            risk_flags,
        };
        validate_cinematic_shot(&shot)?;
        shots.push(shot);
    }
    validate_continuous_ids(shots.iter().map(|shot| shot.shot_id), "镜头分组 shot_id")?;
    if next_global_beat_id <= extracted_beats.len() as i32 {
        return Err(AppError::InvalidArgument(format!(
            "镜头分组必须覆盖全部动作节拍，缺少 beat_id {} 及之后的节拍",
            next_global_beat_id
        )));
    }
    let planning_quality = planning_quality(&shots);
    Ok(VideoShotAnalysis {
        cinematic_shots: shots,
        planning_quality,
    })
}

pub(super) fn parse_stored_video_shot_analysis(
    raw: &str,
    source_spans: &[SourceSpan],
    expected_duration_secs: i32,
) -> Result<VideoShotAnalysis, AppError> {
    let StoredVideoShotAnalysis {
        cinematic_shots, ..
    } = serde_json::from_str(raw.trim())?;
    let mut cinematic_shots = cinematic_shots;
    if cinematic_shots.is_empty() {
        return Err(AppError::InvalidArgument(
            "已存镜头语言分析必须包含至少一个 cinematic_shots 条目".to_string(),
        ));
    }

    let mut seen_ids = HashSet::new();
    for (shot_index, shot) in cinematic_shots.iter_mut().enumerate() {
        let expected_shot_id = (shot_index + 1) as i32;
        let context = format!("已存镜头语言分析 shot_id {}", shot.shot_id);
        if shot.shot_id != expected_shot_id {
            return Err(AppError::InvalidArgument(format!(
                "已存镜头语言分析 shot_id 必须从 1 按数组顺序连续编号，位置 {} 应为 {}，实际为 {}",
                shot_index + 1,
                expected_shot_id,
                shot.shot_id
            )));
        }
        if !seen_ids.insert(shot.shot_id) {
            return Err(AppError::InvalidArgument(format!(
                "已存镜头语言分析 shot_id {} 重复",
                shot.shot_id
            )));
        }
        for (beat_index, beat) in shot.action_beats.iter().enumerate() {
            let expected_beat_id = (beat_index + 1) as i32;
            if beat.beat_id != expected_beat_id {
                return Err(AppError::InvalidArgument(format!(
                    "{context} action_beats.beat_id 必须从 1 按数组顺序连续编号，位置 {} 应为 {}，实际为 {}",
                    beat_index + 1,
                    expected_beat_id,
                    beat.beat_id
                )));
            }
            let expected_beat_type = infer_action_beat_type(&beat.visible_action);
            if beat.beat_type != expected_beat_type {
                return Err(AppError::InvalidArgument(format!(
                    "{context} action_beats beat_id {} 的 beat_type 必须由 visible_action 推导为 {}，实际为 {}",
                    beat.beat_id, expected_beat_type, beat.beat_type
                )));
            }
            validate_source_span_ids(
                beat.source_span_ids.clone(),
                source_spans,
                &format!("{context} action_beats beat_id {}", beat.beat_id),
            )?;
        }
        let derived_source_span_ids = validate_source_span_ids(
            action_beat_source_span_ids(&shot.action_beats),
            source_spans,
            &context,
        )?;
        let derived_source_span_range =
            derive_source_span_range(&derived_source_span_ids, source_spans, &context)?;
        let estimated_natural_seconds = shot
            .action_beats
            .iter()
            .map(|beat| beat.natural_seconds)
            .sum::<f64>();
        let derived_required_segment_count =
            required_segment_count(estimated_natural_seconds, expected_duration_secs)?;
        let derived_risk_flags = shot_risk_flags(
            shot.shot_id,
            &derived_source_span_range,
            &shot.action_beats,
            estimated_natural_seconds,
            derived_required_segment_count,
            expected_duration_secs,
            source_spans,
        )?;
        shot.source_span_ids = derived_source_span_ids.clone();
        shot.source_span_range = derived_source_span_range;
        shot.key_source_span_ids = derived_source_span_ids;
        shot.estimated_natural_seconds = estimated_natural_seconds;
        shot.segment_duration_secs = expected_duration_secs;
        shot.required_segment_count = derived_required_segment_count;
        shot.risk_flags = derived_risk_flags;
        validate_cinematic_shot(shot)?;
    }
    validate_continuous_ids(
        cinematic_shots.iter().map(|shot| shot.shot_id),
        "已存镜头语言分析 shot_id",
    )?;
    let planning_quality = planning_quality(&cinematic_shots);
    Ok(VideoShotAnalysis {
        cinematic_shots,
        planning_quality,
    })
}

pub(super) fn parse_video_frame_prompts_with_defaults(
    raw: &str,
    defaults: &[VideoFramePromptDefaults<'_>],
    prompt_output_language: PromptOutputLanguage,
) -> Result<Vec<VideoFramePrompt>, AppError> {
    parse_video_frame_prompts_inner(raw, defaults, prompt_output_language)
}

fn parse_video_frame_prompts_inner(
    raw: &str,
    defaults: &[VideoFramePromptDefaults<'_>],
    prompt_output_language: PromptOutputLanguage,
) -> Result<Vec<VideoFramePrompt>, AppError> {
    let response: VideoFramePromptResponse = parse_strict_json_response(raw, "视频首尾帧提示词")?;
    if response.frames.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频首尾帧提示词响应必须包含至少一个 frames 条目".to_string(),
        ));
    }
    if defaults.len() != response.frames.len() {
        return Err(AppError::InvalidArgument(format!(
            "视频首尾帧提示词默认字段数量必须等于模型返回帧数：默认 {} 条，实际 {} 条",
            defaults.len(),
            response.frames.len()
        )));
    }
    let mut seen_numbers = HashSet::new();
    let mut frames = Vec::with_capacity(response.frames.len());
    for (index, frame) in response.frames.into_iter().enumerate() {
        let defaults = &defaults[index];
        let frame_number = defaults.frame_number;
        let duration_secs = defaults.duration_secs;
        let aspect_ratio = defaults.aspect_ratio.to_string();
        let reference_image_ids = defaults.reference_image_ids.clone();
        let segment_numbers = defaults.segment_numbers.clone();
        let frame = VideoFramePrompt {
            frame_number,
            title: frame.title,
            scene_summary: frame.scene_summary,
            storyboard_panels: frame.storyboard_panels,
            frame_prompt: frame.frame_prompt,
            blocking_state: frame.blocking_state,
            lighting_state: frame.lighting_state,
            negative_prompt: frame.negative_prompt,
            duration_secs,
            aspect_ratio,
            reference_image_ids,
            segment_numbers,
        };
        let frame = validate_frame_prompt(frame, prompt_output_language)?;
        if !seen_numbers.insert(frame.frame_number) {
            return Err(AppError::InvalidArgument(format!(
                "视频首尾帧提示词 frame_number {} 重复",
                frame.frame_number
            )));
        }
        frames.push(frame);
    }
    Ok(frames)
}

pub(super) fn parse_video_step_object(raw: &str, context: &str) -> Result<Value, AppError> {
    let value: Value = parse_strict_json_response(raw, context)?;
    if !value.is_object() {
        return Err(AppError::InvalidArgument(format!(
            "{context} 响应必须是 JSON 对象"
        )));
    }
    Ok(value)
}

pub(super) fn validate_segment_plan_against_shot_analysis(
    segment_plan: &[VideoSegmentPlan],
    shot_analysis: &VideoShotAnalysis,
) -> Result<(), AppError> {
    if segment_plan.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频分段规划不能为空".to_string(),
        ));
    }
    let shot_by_id = shot_analysis
        .cinematic_shots
        .iter()
        .map(|shot| (shot.shot_id, shot))
        .collect::<HashMap<_, _>>();
    let shot_order = shot_analysis
        .cinematic_shots
        .iter()
        .enumerate()
        .map(|(index, shot)| (shot.shot_id, index))
        .collect::<HashMap<_, _>>();
    let mut covered_by_shot = HashMap::<i32, HashMap<i32, i32>>::new();
    let mut segment_count_by_shot = HashMap::<i32, i32>::new();
    for segment in segment_plan {
        let shot = shot_by_id.get(&segment.analysis_shot_id).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频分段规划 #{} 的 analysis_shot_id {} 不存在",
                segment.segment_number, segment.analysis_shot_id
            ))
        })?;
        if segment.segment_index_in_shot < 1 {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划 #{} 的 segment_index_in_shot 必须大于 0",
                segment.segment_number
            )));
        }
        if segment.shot_segment_count < 1 {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划 #{} 的 shot_segment_count 必须大于 0",
                segment.segment_number
            )));
        }
        *segment_count_by_shot
            .entry(segment.analysis_shot_id)
            .or_default() += 1;
        let allowed_beat_ids = shot
            .action_beats
            .iter()
            .map(|beat| beat.beat_id)
            .collect::<HashSet<_>>();
        let mut seen_in_segment = HashSet::new();
        for beat_id in &segment.covered_beat_ids {
            if *beat_id < 1 {
                return Err(AppError::InvalidArgument(format!(
                    "视频分段规划 #{} covered_beat_ids 不能包含小于 1 的 beat_id",
                    segment.segment_number
                )));
            }
            if !allowed_beat_ids.contains(beat_id) {
                return Err(AppError::InvalidArgument(format!(
                    "视频分段规划 #{} covered_beat_ids 包含不属于镜头 #{} 的 beat_id：{}",
                    segment.segment_number, segment.analysis_shot_id, beat_id
                )));
            }
            if !seen_in_segment.insert(*beat_id) {
                return Err(AppError::InvalidArgument(format!(
                    "视频分段规划 #{} covered_beat_ids 包含重复 beat_id：{}",
                    segment.segment_number, beat_id
                )));
            }
            let count = covered_by_shot
                .entry(segment.analysis_shot_id)
                .or_default()
                .entry(*beat_id)
                .or_default();
            *count += 1;
        }
    }
    validate_segment_plan_shot_order(segment_plan, &shot_order)?;
    validate_segment_plan_shot_counts(segment_plan, &segment_count_by_shot)?;
    for shot in &shot_analysis.cinematic_shots {
        let counts = covered_by_shot.get(&shot.shot_id);
        for beat in &shot.action_beats {
            let count = counts
                .and_then(|items| items.get(&beat.beat_id))
                .copied()
                .unwrap_or(0);
            if beat.required && count != 1 {
                return Err(AppError::InvalidArgument(format!(
                    "视频分段规划必须且只能覆盖镜头 #{} 的必需动作节拍 beat_id {} 一次，实际覆盖 {} 次",
                    shot.shot_id, beat.beat_id, count
                )));
            }
            if count > 1 {
                return Err(AppError::InvalidArgument(format!(
                    "视频分段规划重复覆盖镜头 #{} 的动作节拍 beat_id {}，实际覆盖 {} 次",
                    shot.shot_id, beat.beat_id, count
                )));
            }
        }
    }
    Ok(())
}

fn validate_segment_plan_shot_order(
    segment_plan: &[VideoSegmentPlan],
    shot_order: &HashMap<i32, usize>,
) -> Result<(), AppError> {
    let mut last_order = None;
    let mut closed_shots = HashSet::new();
    let mut active_shot_id = None;
    for segment in segment_plan {
        let order = shot_order
            .get(&segment.analysis_shot_id)
            .copied()
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频分段规划 #{} 的 analysis_shot_id {} 不存在",
                    segment.segment_number, segment.analysis_shot_id
                ))
            })?;
        if let Some(last) = last_order
            && order < last
        {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划 #{} 的 analysis_shot_id {} 不能回到前一个镜头",
                segment.segment_number, segment.analysis_shot_id
            )));
        }
        if active_shot_id != Some(segment.analysis_shot_id) {
            if let Some(previous) = active_shot_id {
                closed_shots.insert(previous);
            }
            if closed_shots.contains(&segment.analysis_shot_id) {
                return Err(AppError::InvalidArgument(format!(
                    "视频分段规划 #{} 的 analysis_shot_id {} 被非连续拆散",
                    segment.segment_number, segment.analysis_shot_id
                )));
            }
            active_shot_id = Some(segment.analysis_shot_id);
        }
        last_order = Some(order);
    }
    Ok(())
}

fn validate_segment_plan_shot_counts(
    segment_plan: &[VideoSegmentPlan],
    segment_count_by_shot: &HashMap<i32, i32>,
) -> Result<(), AppError> {
    let mut next_index_by_shot = HashMap::<i32, i32>::new();
    for segment in segment_plan {
        let actual_count = segment_count_by_shot
            .get(&segment.analysis_shot_id)
            .copied()
            .unwrap_or(0);
        if segment.shot_segment_count != actual_count {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划 #{} 的 shot_segment_count 必须等于镜头 #{} 实际 segment 数量 {}，实际为 {}",
                segment.segment_number,
                segment.analysis_shot_id,
                actual_count,
                segment.shot_segment_count
            )));
        }
        let expected_index = next_index_by_shot
            .entry(segment.analysis_shot_id)
            .and_modify(|index| *index += 1)
            .or_insert(1);
        if segment.segment_index_in_shot != *expected_index {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划 #{} 的 segment_index_in_shot 必须是镜头 #{} 内连续编号 {}，实际为 {}",
                segment.segment_number,
                segment.analysis_shot_id,
                *expected_index,
                segment.segment_index_in_shot
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_video_prompt_shot_against_segment(
    shot: VideoPromptShot,
    segment: &VideoSegmentPlan,
    prompt_output_language: PromptOutputLanguage,
) -> Result<VideoPromptShot, AppError> {
    validate_shot(shot, &segment.source_span_ids, prompt_output_language)
}

fn validate_model_video_prompt_shot_against_segment(
    shot: VideoPromptShot,
    segment: &VideoSegmentPlan,
    prompt_output_language: PromptOutputLanguage,
) -> Result<VideoPromptShot, AppError> {
    validate_model_shot(shot, &segment.source_span_ids, prompt_output_language)
}

fn validate_source_span_ids(
    ids: Vec<String>,
    source_spans: &[SourceSpan],
    context: &str,
) -> Result<Vec<String>, AppError> {
    if ids.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{context} source_span_ids 不能为空"
        )));
    }
    let allowed = source_spans
        .iter()
        .map(|span| span.id.as_str())
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    let mut normalized = Vec::with_capacity(ids.len());
    for id in ids {
        let id = id.trim();
        if id.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "{context} source_span_ids 不能包含空 ID"
            )));
        }
        if !allowed.contains(id) {
            return Err(AppError::InvalidArgument(format!(
                "{context} source_span_ids 包含不在本次输入候选中的 ID：{id}"
            )));
        }
        if !seen.insert(id.to_string()) {
            return Err(AppError::InvalidArgument(format!(
                "{context} source_span_ids 包含重复 ID：{id}"
            )));
        }
        normalized.push(id.to_string());
    }
    Ok(normalized)
}

fn source_span_index(id: &str, source_spans: &[SourceSpan]) -> Option<usize> {
    source_spans.iter().position(|span| span.id == id)
}

fn required_segment_count(
    estimated_natural_seconds: f64,
    expected_duration_secs: i32,
) -> Result<i32, AppError> {
    if expected_duration_secs < 1 {
        return Err(AppError::InvalidArgument(
            "用户选择的视频片段时长必须大于 0".to_string(),
        ));
    }
    if !estimated_natural_seconds.is_finite() || estimated_natural_seconds <= 0.0 {
        return Err(AppError::InvalidArgument(
            "镜头语言分析 action_beats.natural_seconds 合计必须大于 0".to_string(),
        ));
    }
    Ok((estimated_natural_seconds / f64::from(expected_duration_secs)).ceil() as i32)
}

fn planning_quality(shots: &[VideoCinematicShot]) -> VideoPlanningQuality {
    let total_estimated_seconds = shots
        .iter()
        .map(|shot| shot.estimated_natural_seconds)
        .sum::<f64>();
    let total_required_segments = shots
        .iter()
        .map(|shot| shot.required_segment_count)
        .sum::<i32>();
    let risk_flags = shots
        .iter()
        .flat_map(|shot| shot.risk_flags.iter().cloned())
        .collect::<Vec<_>>();
    VideoPlanningQuality {
        total_estimated_seconds,
        total_required_segments,
        risk_flags,
    }
}

fn shot_risk_flags(
    shot_id: i32,
    span_range: &VideoSourceSpanRange,
    action_beats: &[VideoActionBeat],
    estimated_natural_seconds: f64,
    required_segment_count: i32,
    expected_duration_secs: i32,
    source_spans: &[SourceSpan],
) -> Result<Vec<VideoPlanningRisk>, AppError> {
    let mut risks = Vec::new();
    let start_index = source_span_index(&span_range.start, source_spans).ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "镜头语言分析 shot_id {shot_id} source_span_range.start 无效"
        ))
    })?;
    let end_index = source_span_index(&span_range.end, source_spans).ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "镜头语言分析 shot_id {shot_id} source_span_range.end 无效"
        ))
    })?;
    let span_width = end_index.saturating_sub(start_index) + 1;
    if span_width > (required_segment_count as usize).saturating_mul(8).max(12) {
        risks.push(planning_risk(
            "warning",
            "wide_source_span",
            Some(shot_id),
            "该镜头覆盖正文跨度较宽，后续片段可能动作密度过高",
        ));
    }
    if action_beats.len() > (required_segment_count as usize).saturating_mul(3).max(3) {
        risks.push(planning_risk(
            "warning",
            "dense_action_beats",
            Some(shot_id),
            "该镜头动作节拍数量偏多，片段规划需要避免过度压缩",
        ));
    }
    if action_beats
        .iter()
        .any(|beat| beat.natural_seconds > f64::from(expected_duration_secs))
    {
        risks.push(planning_risk(
            "warning",
            "long_action_beat",
            Some(shot_id),
            "存在单个动作节拍长于用户选择的片段时长，后续可能需要拆得更细",
        ));
    }
    if estimated_natural_seconds > f64::from(required_segment_count * expected_duration_secs) {
        risks.push(planning_risk(
            "warning",
            "optimistic_timing",
            Some(shot_id),
            "该镜头自然时长接近或超过片段预算上限",
        ));
    }
    Ok(risks)
}

fn planning_risk(
    level: &str,
    code: &str,
    shot_id: Option<i32>,
    message: &str,
) -> VideoPlanningRisk {
    VideoPlanningRisk {
        level: level.to_string(),
        code: code.to_string(),
        shot_id,
        message: message.to_string(),
    }
}

fn validate_cinematic_shot(shot: &VideoCinematicShot) -> Result<(), AppError> {
    if shot.shot_id < 1 {
        return Err(AppError::InvalidArgument(
            "镜头语言分析 shot_id 必须大于 0".to_string(),
        ));
    }
    if !shot.estimated_natural_seconds.is_finite() || shot.estimated_natural_seconds <= 0.0 {
        return Err(AppError::InvalidArgument(format!(
            "镜头语言分析 shot_id {} 的 estimated_natural_seconds 必须大于 0",
            shot.shot_id
        )));
    }
    if shot.required_segment_count < 1 {
        return Err(AppError::InvalidArgument(format!(
            "镜头语言分析 shot_id {} 的 required_segment_count 必须大于 0",
            shot.shot_id
        )));
    }
    if shot.action_beats.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "镜头语言分析 shot_id {} 的 action_beats 不能为空",
            shot.shot_id
        )));
    }
    match shot.action_load.as_str() {
        "light" | "medium" | "heavy" => {}
        other => {
            return Err(AppError::InvalidArgument(format!(
                "镜头语言分析 shot_id {} 的 action_load 无效：{}",
                shot.shot_id, other
            )));
        }
    }
    validate_continuous_ids(
        shot.action_beats.iter().map(|item| item.beat_id),
        &format!("镜头语言分析 shot_id {} 的 beat_id", shot.shot_id),
    )?;
    let beat_seconds = shot
        .action_beats
        .iter()
        .map(|beat| {
            if !beat.natural_seconds.is_finite() || beat.natural_seconds <= 0.0 {
                return Err(AppError::InvalidArgument(format!(
                    "镜头语言分析 shot_id {} action_beats beat_id {} 的 natural_seconds 必须大于 0",
                    shot.shot_id, beat.beat_id
                )));
            }
            Ok(beat.natural_seconds)
        })
        .collect::<Result<Vec<_>, AppError>>()?
        .into_iter()
        .sum::<f64>();
    if (beat_seconds - shot.estimated_natural_seconds).abs() > 0.001 {
        return Err(AppError::InvalidArgument(format!(
            "镜头语言分析 shot_id {} 的 estimated_natural_seconds 必须由 action_beats.natural_seconds 汇总得到",
            shot.shot_id
        )));
    }
    Ok(())
}

fn infer_action_beat_type(visible_action: &str) -> &'static str {
    let text = visible_action.trim().to_ascii_lowercase();
    if contains_any(
        &text,
        &[
            "镜头", "机位", "推近", "拉远", "摇移", "俯拍", "仰拍", "camera", "dolly", "pan",
            "tilt", "zoom",
        ],
    ) {
        return "camera";
    }
    if contains_any(
        &text,
        &[
            "切入",
            "切出",
            "转场",
            "淡入",
            "淡出",
            "闪回",
            "过渡",
            "cut",
            "transition",
        ],
    ) {
        return "transition";
    }
    if contains_any(
        &text,
        &[
            "说", "问", "喊", "念", "回答", "低语", "开口", "闭口", "口型", "对白", "台词",
            "dialogue", "speak", "say", "ask",
        ],
    ) {
        return "dialogue";
    }
    if contains_any(
        &text,
        &[
            "拿", "放", "推门", "关门", "拾起", "握住", "触碰", "翻开", "打开", "合上", "道具",
            "门", "书", "杯", "刀", "枪", "object", "prop",
        ],
    ) {
        return "object_interaction";
    }
    if contains_any(
        &text,
        &[
            "停顿", "静止", "屏住", "沉默", "暂停", "凝住", "pause", "hold", "still",
        ],
    ) {
        return "pause";
    }
    if contains_any(
        &text,
        &[
            "表情",
            "皱眉",
            "睁大",
            "抬眼",
            "低头",
            "看见",
            "望向",
            "回头",
            "反应",
            "意识到",
            "reaction",
            "glance",
            "look",
        ],
    ) {
        return "reaction";
    }
    if contains_any(
        &text,
        &[
            "变得", "转为", "收紧", "放松", "改变", "稳定", "恢复", "状态", "state", "change",
        ],
    ) {
        return "state_change";
    }
    "movement"
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn validate_continuous_ids<I>(ids: I, label: &str) -> Result<(), AppError>
where
    I: IntoIterator<Item = i32>,
{
    for (index, id) in ids.into_iter().enumerate() {
        let expected = (index + 1) as i32;
        if id != expected {
            return Err(AppError::InvalidArgument(format!(
                "{label} 必须从 1 连续编号，位置 {} 应为 {}，实际为 {}",
                index + 1,
                expected,
                id
            )));
        }
    }
    Ok(())
}

fn required_non_empty_text(value: String, field: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field} 不能为空")));
    }
    Ok(value.to_string())
}

fn required_non_empty_value(value: Value, field: &str) -> Result<Value, AppError> {
    let valid = match &value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(object) => !object.is_empty(),
        _ => true,
    };
    if valid {
        Ok(value)
    } else {
        Err(AppError::InvalidArgument(format!("{field} 不能为空")))
    }
}

fn ordered_segment_numbers(segment_by_number: &HashMap<i32, &VideoSegmentPlan>) -> Vec<i32> {
    let mut numbers = segment_by_number.keys().copied().collect::<Vec<_>>();
    numbers.sort_unstable();
    numbers
}

fn derived_prompt_shot_number(
    index: usize,
    item_count: usize,
    ordered_segment_numbers: &[i32],
    field: &str,
) -> Result<i32, AppError> {
    if item_count == 1 && ordered_segment_numbers.len() == 1 {
        return Ok(ordered_segment_numbers[0]);
    }
    if item_count == ordered_segment_numbers.len() {
        return ordered_segment_numbers
            .get(index)
            .copied()
            .ok_or_else(|| AppError::InvalidArgument(format!("{field} 无法按片段顺序推导")));
    }
    Err(AppError::InvalidArgument(format!(
        "{field} 缺失，且无法从当前批次和片段数量唯一推导"
    )))
}

fn derive_segment_source_span_ids(
    shot_analysis: &VideoShotAnalysis,
    analysis_shot_id: i32,
    covered_beat_ids: &[i32],
    segment_number: i32,
) -> Result<Vec<String>, AppError> {
    let shot = shot_analysis
        .cinematic_shots
        .iter()
        .find(|shot| shot.shot_id == analysis_shot_id)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频分段规划 segment_number {} 的 analysis_shot_id {} 不存在，无法推导 source_span_ids",
                segment_number, analysis_shot_id
            ))
        })?;
    let covered = covered_beat_ids.iter().copied().collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    for beat in &shot.action_beats {
        if !covered.contains(&beat.beat_id) {
            continue;
        }
        for id in &beat.source_span_ids {
            if seen.insert(id.clone()) {
                ids.push(id.clone());
            }
        }
    }
    if ids.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "视频分段规划 segment_number {} 无法从 covered_beat_ids 推导 source_span_ids",
            segment_number
        )));
    }
    Ok(ids)
}

fn action_beat_source_span_ids(action_beats: &[VideoActionBeat]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    for beat in action_beats {
        for id in &beat.source_span_ids {
            if seen.insert(id.clone()) {
                ids.push(id.clone());
            }
        }
    }
    ids
}

fn derive_source_span_range(
    ids: &[String],
    source_spans: &[SourceSpan],
    context: &str,
) -> Result<VideoSourceSpanRange, AppError> {
    let mut indexes = Vec::with_capacity(ids.len());
    for id in ids {
        let index = source_span_index(id, source_spans).ok_or_else(|| {
            AppError::InvalidArgument(format!("{context} source_span_ids 包含无效 ID：{id}"))
        })?;
        indexes.push((index, id));
    }
    let Some((_, start)) = indexes.iter().min_by_key(|(index, _)| *index) else {
        return Err(AppError::InvalidArgument(format!(
            "{context} 无法从空 source_span_ids 推导 source_span_range"
        )));
    };
    let Some((_, end)) = indexes.iter().max_by_key(|(index, _)| *index) else {
        return Err(AppError::InvalidArgument(format!(
            "{context} 无法从空 source_span_ids 推导 source_span_range"
        )));
    };
    Ok(VideoSourceSpanRange {
        start: (*start).clone(),
        end: (*end).clone(),
    })
}

#[cfg(test)]
mod tests;
