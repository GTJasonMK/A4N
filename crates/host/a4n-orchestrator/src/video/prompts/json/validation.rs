use std::collections::HashSet;

use a4n_error::AppError;
use serde_json::Value;

mod frame;
mod prompt_text;
mod segment;
mod shot;

pub(super) use frame::validate_frame_prompt;
pub(in crate::video) use prompt_text::direct_prompt_internal_reference_term;
use prompt_text::required_text;
pub(super) use prompt_text::validate_inserted_segment_draft;
pub use prompt_text::{
    validate_direct_prompt_text, validate_reference_label_text,
    video_motion_prompt_has_temporal_storyboard,
    video_prompt_missing_required_section_for_language, video_prompt_stale_section_for_language,
    video_prompt_upload_status_term,
};
pub(super) use prompt_text::{
    validate_prompt_has_no_upload_status, validate_prompt_has_shot_storyboard_section,
};
pub(in crate::video) use segment::validate_segment_plan;
pub(super) use shot::{validate_model_shot, validate_shot};

#[derive(Debug, Clone, Copy)]
pub(super) enum PromptTextKind {
    VideoVisual,
    VideoMotion,
    FramePrompt,
    FrameNegative,
}

pub(in crate::video) const API_PROMPT_UPLOAD_STATUS_TERMS: &[&str] = &[
    "无输入首帧",
    "没有输入首帧",
    "无输入图片",
    "没有输入图片",
    "无输入尾帧",
    "没有输入尾帧",
    "无首帧",
    "没有首帧",
    "无首帧图",
    "没有首帧图",
    "无尾帧",
    "没有尾帧",
    "无尾帧图",
    "没有尾帧图",
    "无上传图片",
    "本段无上传图片",
    "没有上传图片",
    "无参考图",
    "没有参考图",
    "仅依赖文字描述",
];

pub(super) fn validate_structured_value(value: &Value, field: &str) -> Result<(), AppError> {
    match value {
        Value::Null => Err(AppError::InvalidArgument(format!("{field} 不能为空"))),
        Value::String(text) if text.trim().is_empty() => {
            Err(AppError::InvalidArgument(format!("{field} 不能为空")))
        }
        Value::Array(items) if items.is_empty() => {
            Err(AppError::InvalidArgument(format!("{field} 不能为空")))
        }
        Value::Object(map) if map.is_empty() => {
            Err(AppError::InvalidArgument(format!("{field} 不能为空")))
        }
        _ => Ok(()),
    }
}

pub(super) fn validate_duration(duration_secs: i32, field: &str) -> Result<(), AppError> {
    if !(1..=60).contains(&duration_secs) {
        return Err(AppError::InvalidArgument(format!(
            "{field} 必须在 1 到 60 秒之间"
        )));
    }
    Ok(())
}

pub(super) fn validate_reference_ids(ids: &[i64], field: &str) -> Result<(), AppError> {
    let mut seen = HashSet::with_capacity(ids.len());
    for (index, id) in ids.iter().copied().enumerate() {
        if id <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "{field}[{index}] 必须为正整数"
            )));
        }
        if !seen.insert(id) {
            return Err(AppError::InvalidArgument(format!(
                "{field} 包含重复 ID：{id}"
            )));
        }
    }
    Ok(())
}
