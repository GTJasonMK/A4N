#[cfg(test)]
use a4n_domain::prompt_language::PromptOutputLanguage;
use a4n_domain::source_spans::SourceSpan;
use a4n_error::AppError;
use serde_json::Value;
use std::collections::HashSet;

mod chat_prompts;
mod contracts;
mod cross_validation;
mod derived_fields;
mod frame_parser;
mod segment_parser;
mod shot_parser;
mod validation;

pub(in crate::video) use self::contracts::*;

pub(in crate::video) use self::chat_prompts::{
    ChatVideoPromptShot, parse_chat_video_prompt_shots,
    validate_chat_video_prompt_shot_against_segment, validate_chat_video_prompt_text,
};
pub(in crate::video) use self::cross_validation::{
    validate_model_video_prompt_shot_against_segment,
    validate_segment_plan_against_shooting_script_projection,
    validate_video_prompt_shot_against_segment,
};
pub(in crate::video) use self::frame_parser::{
    parse_storyboard_frame_prompts_with_defaults, parse_video_frame_prompts_with_defaults,
};
pub(in crate::video) use self::segment_parser::{
    parse_video_segment_plan_response, validate_stored_video_segment_plan,
    video_segment_plan_input_values,
};
pub(in crate::video) use self::shot_parser::{
    derived_prompt_shot_number, ordered_segment_numbers, parse_video_prompt_shots,
};
pub(in crate::video) use self::validation::direct_prompt_internal_reference_term;
pub(in crate::video) use self::validation::API_PROMPT_UPLOAD_STATUS_TERMS;
use self::validation::validate_frame_prompt;
pub(in crate::video) use self::validation::validate_segment_plan;
pub use self::validation::{
    validate_direct_prompt_text, validate_reference_label_text,
    video_motion_prompt_has_temporal_storyboard,
    video_prompt_missing_required_section_for_language, video_prompt_stale_section_for_language,
    video_prompt_upload_status_term,
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

pub(in crate::video) fn parse_strict_json_response<T>(
    raw: &str,
    context: &str,
) -> Result<T, AppError>
where
    T: serde::de::DeserializeOwned,
{
    crate::structured_llm::parse_strict_json_response(raw, context)
}

pub(in crate::video) fn validate_source_span_ids(
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

pub(in crate::video) fn derive_segment_source_span_ids(
    shooting_script_projection: &VideoShootingScriptProjection,
    source_shot_id: &str,
    covered_beat_ids: &[String],
    segment_number: i32,
) -> Result<Vec<String>, AppError> {
    let shot = shooting_script_projection
        .shot(source_shot_id)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频分段规划 segment_number {} 的 source_shot_id {} 不存在，无法推导 source_span_ids",
                segment_number, source_shot_id
            ))
        })?;
    let covered = covered_beat_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    if shot.action_beats.is_empty() && covered.is_empty() {
        return Ok(shot.source_span_ids.clone());
    }
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    for beat in &shot.action_beats {
        if !covered.contains(beat.beat_id.as_str()) {
            continue;
        }
        for id in &beat.source_span_ids {
            if seen.insert(id.clone()) {
                ids.push(id.clone());
            }
        }
    }
    if ids.is_empty() && shot.director_constraints.is_none() {
        return Err(AppError::InvalidArgument(format!(
            "视频分段规划 segment_number {} 无法从 covered_beat_ids 推导 source_span_ids",
            segment_number
        )));
    }
    Ok(ids)
}

#[cfg(test)]
mod tests;
