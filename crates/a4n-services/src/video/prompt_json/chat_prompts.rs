use std::collections::{HashMap, HashSet};

use a4n_core::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::prompt_language::PromptOutputLanguage;

use super::derived_fields::derive_chat_video;
use super::validation::{
    validate_prompt_has_no_upload_status, validate_prompt_has_shot_storyboard_section,
    validate_reference_ids,
};
use super::{
    VideoSegmentPlan, derived_prompt_shot_number, ordered_segment_numbers,
    validate_direct_prompt_text,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(in crate::video) struct ChatVideoPromptShot {
    pub shot_number: i32,
    pub title: String,
    pub scene_summary: String,
    pub chat_prompt: String,
    pub negative_prompt: String,
    pub chat_video: Value,
    pub duration_secs: i32,
    pub aspect_ratio: String,
    pub reference_image_ids: Vec<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChatVideoPromptShotInput {
    pub title: String,
    pub scene_summary: String,
    pub chat_prompt: String,
    pub negative_prompt: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChatVideoPromptResponse {
    shots: Vec<ChatVideoPromptShotInput>,
}

pub(in crate::video) fn parse_chat_video_prompt_shots(
    raw: &str,
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<Vec<ChatVideoPromptShot>, AppError> {
    let response: ChatVideoPromptResponse = serde_json::from_str(raw.trim())?;
    if response.shots.is_empty() {
        return Err(AppError::InvalidArgument(
            "Chat 视频提示词响应必须包含至少一个 shots 条目".to_string(),
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
            "Chat 视频提示词 shot_number",
        )?;
        let segment = segment_by_number.get(&shot_number).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "Chat 视频提示词 shot_number {} 必须对应已有片段编号",
                shot_number
            ))
        })?;
        let negative_prompt = input.negative_prompt.trim().to_string();
        validate_direct_prompt_text(&negative_prompt, "Chat 视频提示词 negative_prompt")?;
        let shot = ChatVideoPromptShot {
            shot_number,
            title: required_text(input.title, "title")?,
            scene_summary: required_text(input.scene_summary, "scene_summary")?,
            chat_prompt: required_chat_prompt(input.chat_prompt, prompt_output_language)?,
            negative_prompt,
            chat_video: derive_chat_video(segment),
            duration_secs: segment.duration_secs,
            aspect_ratio: segment.aspect_ratio.clone(),
            reference_image_ids: segment.reference_image_ids.clone(),
        };
        let shot =
            validate_chat_video_prompt_shot_against_segment(shot, segment, prompt_output_language)
                .map_err(|err| match err {
                    AppError::InvalidArgument(message) => AppError::InvalidArgument(format!(
                        "Chat 视频提示词 shot_number {} 无效：{}",
                        shot_number, message
                    )),
                    other => other,
                })?;
        if !seen_numbers.insert(shot.shot_number) {
            return Err(AppError::InvalidArgument(format!(
                "Chat 视频提示词 shot_number {} 重复",
                shot.shot_number
            )));
        }
        shots.push(shot);
    }
    Ok(shots)
}

pub(in crate::video) fn validate_chat_video_prompt_shot_against_segment(
    shot: ChatVideoPromptShot,
    segment: &VideoSegmentPlan,
    prompt_output_language: PromptOutputLanguage,
) -> Result<ChatVideoPromptShot, AppError> {
    validate_direct_prompt_text(&shot.chat_prompt, "Chat 视频提示词 chat_prompt")?;
    validate_prompt_has_no_upload_status(&shot.chat_prompt, "Chat 视频提示词 chat_prompt")?;
    validate_chat_prompt_required_sections(&shot.chat_prompt, prompt_output_language)?;
    validate_reference_ids(
        &shot.reference_image_ids,
        "Chat 视频提示词 reference_image_ids",
    )?;
    validate_chat_video_source_span_ids(&shot.chat_video, &segment.source_span_ids)?;
    validate_chat_video_input_images(&shot.chat_video, &shot.reference_image_ids)?;
    Ok(shot)
}

fn required_text(value: String, field: &str) -> Result<String, AppError> {
    let text = value.trim();
    if text.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "Chat 视频提示词 {field} 不能为空"
        )));
    }
    Ok(text.to_string())
}

fn required_chat_prompt(
    value: String,
    prompt_output_language: PromptOutputLanguage,
) -> Result<String, AppError> {
    let text = required_text(value, "chat_prompt")?;
    validate_direct_prompt_text(&text, "Chat 视频提示词 chat_prompt")?;
    validate_prompt_has_no_upload_status(&text, "Chat 视频提示词 chat_prompt")?;
    validate_chat_prompt_required_sections(&text, prompt_output_language)?;
    validate_prompt_has_shot_storyboard_section(
        &text,
        "Chat 视频提示词 chat_prompt",
        prompt_output_language,
    )?;
    let avoidance_section = prompt_output_language
        .visual_sections()
        .last()
        .and_then(|section| chat_prompt_section(&text, section))
        .unwrap_or("");
    if !contains_no_background_music_rule(avoidance_section) {
        return Err(AppError::InvalidArgument(
            "Chat 视频提示词 chat_prompt 的限制与避免项必须明确包含不要配背景音乐/配乐/BGM/background music 的限制"
                .to_string(),
        ));
    }
    Ok(text)
}

fn validate_chat_prompt_required_sections(
    text: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    for section in prompt_output_language.visual_sections() {
        if !text.contains(section) {
            return Err(AppError::InvalidArgument(format!(
                "Chat 视频提示词 chat_prompt 缺少章节 {section}"
            )));
        }
    }
    Ok(())
}

fn chat_prompt_section<'a>(text: &'a str, section: &str) -> Option<&'a str> {
    let start = text.find(section)?;
    let content_start = start + section.len();
    let rest = &text[content_start..];
    let end = rest.find("\n[").unwrap_or(rest.len());
    Some(rest[..end].trim())
}

fn contains_no_background_music_rule(text: &str) -> bool {
    let lowercase = text.to_ascii_lowercase();
    let no_music_phrases = [
        "不要配背景音乐",
        "不要添加背景音乐",
        "不要加背景音乐",
        "禁止背景音乐",
        "无背景音乐",
        "不要背景音乐",
        "不要配乐",
        "禁止配乐",
        "无配乐",
        "不要加配乐",
        "no background music",
        "without background music",
        "do not add background music",
        "do not include background music",
        "no soundtrack",
        "without soundtrack",
        "no bgm",
    ];
    no_music_phrases
        .iter()
        .any(|phrase| text.contains(phrase) || lowercase.contains(phrase))
        || ((lowercase.contains("bgm") || lowercase.contains("background music"))
            && [
                "不要",
                "禁止",
                "无",
                "不要加",
                "不要添加",
                "不要配",
                "no",
                "without",
                "do not",
                "don't",
            ]
            .iter()
            .any(|prefix| text.contains(prefix) || lowercase.contains(prefix)))
}

fn validate_chat_video_source_span_ids(
    chat_video: &Value,
    allowed_source_span_ids: &[String],
) -> Result<(), AppError> {
    let source_span_ids = chat_video
        .get("source_span_ids")
        .ok_or_else(|| {
            AppError::InvalidArgument("chat_video.source_span_ids 不能为空".to_string())
        })?
        .as_array()
        .ok_or_else(|| {
            AppError::InvalidArgument("chat_video.source_span_ids 必须是数组".to_string())
        })?;
    if source_span_ids.is_empty() {
        return Err(AppError::InvalidArgument(
            "chat_video.source_span_ids 不能为空".to_string(),
        ));
    }
    let allowed = allowed_source_span_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    for (index, value) in source_span_ids.iter().enumerate() {
        let id = value
            .as_str()
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "chat_video.source_span_ids[{}] 必须是非空字符串",
                    index + 1
                ))
            })?;
        if !allowed.contains(id) {
            return Err(AppError::InvalidArgument(format!(
                "chat_video.source_span_ids[{}] 不属于当前片段 source_span_ids：{id}",
                index + 1
            )));
        }
        if !seen.insert(id) {
            return Err(AppError::InvalidArgument(format!(
                "chat_video.source_span_ids[{}] 重复：{id}",
                index + 1
            )));
        }
    }
    Ok(())
}

fn validate_chat_video_input_images(
    chat_video: &Value,
    reference_image_ids: &[i64],
) -> Result<(), AppError> {
    let Some(input_images_value) = chat_video.get("input_images") else {
        if reference_image_ids.is_empty() {
            return Ok(());
        }
        return Err(AppError::InvalidArgument(
            "chat_video.input_images 必须按 reference_image_ids 记录每张 Chat 上传图片".to_string(),
        ));
    };
    let input_images = input_images_value.as_array().ok_or_else(|| {
        AppError::InvalidArgument("chat_video.input_images 必须是数组".to_string())
    })?;
    if input_images.len() != reference_image_ids.len() {
        return Err(AppError::InvalidArgument(format!(
            "chat_video.input_images 数量必须等于 reference_image_ids 数量：当前 {}，期望 {}",
            input_images.len(),
            reference_image_ids.len()
        )));
    }
    for (index, (input_image, expected_image_id)) in
        input_images.iter().zip(reference_image_ids).enumerate()
    {
        let order = index + 1;
        let object = input_image.as_object().ok_or_else(|| {
            AppError::InvalidArgument(format!("chat_video.input_images[{order}] 必须是对象"))
        })?;
        let image_id = object
            .get("image_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "chat_video.input_images[{order}].image_id 必须是整数"
                ))
            })?;
        if image_id != *expected_image_id {
            return Err(AppError::InvalidArgument(format!(
                "chat_video.input_images[{order}].image_id 必须等于 reference_image_ids[{order}]：期望 {}，实际 {}",
                expected_image_id, image_id
            )));
        }
        let upload_order = object
            .get("upload_order")
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "chat_video.input_images[{order}].upload_order 必须是整数"
                ))
            })?;
        if upload_order != order as i64 {
            return Err(AppError::InvalidArgument(format!(
                "chat_video.input_images[{order}].upload_order 必须等于 {order}，实际 {upload_order}"
            )));
        }
        for field in ["role", "usage"] {
            let text = object
                .get(field)
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            if text.is_empty() {
                return Err(AppError::InvalidArgument(format!(
                    "chat_video.input_images[{order}].{field} 必须是非空字符串"
                )));
            }
        }
    }
    Ok(())
}
