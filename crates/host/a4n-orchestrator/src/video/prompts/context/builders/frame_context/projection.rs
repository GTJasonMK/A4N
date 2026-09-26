use a4n_error::AppError;
use serde_json::{Value, json};

use super::super::rendering::chapter_analysis_context;
use super::segments::{segment_number, segment_plan_items};

pub(in crate::video::prompts::context::builders) fn lightweight_chapter_context(
    chapter_summary: Option<&str>,
    chapter_analysis: Option<&Value>,
) -> Value {
    json!({
        "summary": chapter_summary.map(str::trim).filter(|s| !s.is_empty()),
        "analysis": chapter_analysis_context(chapter_analysis)
    })
}

pub(in crate::video::prompts::context::builders) fn frame_prompt_context(
    segment_plan: &Value,
    current_frame: &Value,
) -> Value {
    let expected_segment_numbers = current_frame
        .get("expected_segment_numbers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_i64)
        .collect::<Vec<_>>();
    let segments = segment_plan_items(segment_plan);
    let relevant_segments = segments
        .into_iter()
        .filter(|segment| {
            segment_number(segment).is_some_and(|number| expected_segment_numbers.contains(&number))
        })
        .cloned()
        .collect::<Vec<_>>();
    json!({
        "relevant_segments": relevant_segments,
    })
}

pub(in crate::video::prompts::context::builders) fn required_current_frame_field(
    current_frame: &Value,
    key: &str,
    expected: &str,
) -> Result<Value, AppError> {
    let value = current_frame.get(key).ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "视频边界帧提示词请求构造失败：current_frame 缺少系统字段 {key}"
        ))
    })?;
    if value.is_null() {
        return Err(AppError::InvalidArgument(format!(
            "视频边界帧提示词请求构造失败：current_frame.{key} 不能为 null"
        )));
    }
    match expected {
        "integer" if !value.is_i64() && !value.is_u64() => Err(AppError::InvalidArgument(format!(
            "视频边界帧提示词请求构造失败：current_frame.{key} 必须是整数"
        ))),
        "array" if value.as_array().is_none_or(|items| items.is_empty()) => {
            Err(AppError::InvalidArgument(format!(
                "视频边界帧提示词请求构造失败：current_frame.{key} 必须是非空数组"
            )))
        }
        "string" if value.as_str().is_none_or(|text| text.trim().is_empty()) => {
            Err(AppError::InvalidArgument(format!(
                "视频边界帧提示词请求构造失败：current_frame.{key} 必须是非空字符串"
            )))
        }
        _ => Ok(value.clone()),
    }
}
