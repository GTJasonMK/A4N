use a4n_core::error::AppError;
use serde_json::Value;

use super::super::validate_structured_value;

pub(super) fn validate_self_contained_context(
    value: &Value,
    shot_number: i32,
) -> Result<(), AppError> {
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument("视频提示词 self_contained_context 必须是对象".to_string())
    })?;
    if object.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词 self_contained_context 不能为空".to_string(),
        ));
    }
    for key in [
        "must_repeat_from_previous",
        "must_establish_in_segment",
        "must_preserve_throughout",
        "handoff_to_next",
        "omission_risks",
    ] {
        if shot_number == 1 && key == "must_repeat_from_previous" && object.contains_key(key) {
            continue;
        }
        validate_structured_value(
            object.get(key).unwrap_or(&Value::Null),
            &format!("视频提示词 self_contained_context.{key}"),
        )?;
    }
    Ok(())
}

pub(super) fn validate_transition_in(value: &Value) -> Result<(), AppError> {
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument("视频提示词 transition_in 必须是对象".to_string())
    })?;
    if object.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词 transition_in 不能为空".to_string(),
        ));
    }
    for key in [
        "from_previous_end",
        "to_current_start",
        "cut_type",
        "camera_transition",
        "first_beat_action",
    ] {
        validate_structured_value(
            object.get(key).unwrap_or(&Value::Null),
            &format!("视频提示词 transition_in.{key}"),
        )?;
    }
    Ok(())
}
