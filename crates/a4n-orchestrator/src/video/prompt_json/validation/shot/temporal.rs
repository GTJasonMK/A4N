use a4n_core::error::AppError;
use serde_json::Value;

use super::super::validate_structured_value;

pub(super) fn validate_temporal_beats(value: &Value, duration_secs: i32) -> Result<(), AppError> {
    let beats = value.as_array().ok_or_else(|| {
        AppError::InvalidArgument("视频提示词 temporal_beats 必须是数组".to_string())
    })?;
    if beats.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词 temporal_beats 不能为空".to_string(),
        ));
    }

    let mut expected_start = 0.0_f64;
    for (index, beat) in beats.iter().enumerate() {
        let object = beat.as_object().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频提示词 temporal_beats[{}] 必须是对象",
                index + 1
            ))
        })?;
        let start_sec = object
            .get("start_sec")
            .and_then(Value::as_f64)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频提示词 temporal_beats[{}].start_sec 必须是数字",
                    index + 1
                ))
            })?;
        let end_sec = object
            .get("end_sec")
            .and_then(Value::as_f64)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频提示词 temporal_beats[{}].end_sec 必须是数字",
                    index + 1
                ))
            })?;
        if (start_sec - expected_start).abs() > 0.05 {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词 temporal_beats[{}].start_sec 必须衔接上一段 end_sec",
                index + 1
            )));
        }
        if end_sec <= start_sec {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词 temporal_beats[{}].end_sec 必须大于 start_sec",
                index + 1
            )));
        }
        if end_sec - f64::from(duration_secs) > 0.05 {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词 temporal_beats[{}].end_sec 不能超过 duration_secs",
                index + 1
            )));
        }
        validate_structured_value(
            object.get("camera").unwrap_or(&Value::Null),
            &format!("视频提示词 temporal_beats[{}].camera", index + 1),
        )?;
        validate_structured_value(
            object.get("subject_action").unwrap_or(&Value::Null),
            &format!("视频提示词 temporal_beats[{}].subject_action", index + 1),
        )?;
        validate_structured_value(
            object.get("continuity_target").unwrap_or(&Value::Null),
            &format!("视频提示词 temporal_beats[{}].continuity_target", index + 1),
        )?;
        expected_start = end_sec;
    }

    if (expected_start - f64::from(duration_secs)).abs() > 0.05 {
        return Err(AppError::InvalidArgument(
            "视频提示词 temporal_beats 最后一段 end_sec 必须等于 duration_secs".to_string(),
        ));
    }
    Ok(())
}
