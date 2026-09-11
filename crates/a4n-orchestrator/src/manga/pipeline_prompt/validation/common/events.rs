use std::collections::HashSet;

use a4n_error::AppError;
use serde_json::Value;

pub(in crate::manga::pipeline_prompt::validation) fn validate_manga_nonnegative_index(
    value: Option<&Value>,
    path: &str,
) -> Result<i64, AppError> {
    let index = value
        .and_then(Value::as_i64)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path} 必须是非负整数")))?;
    if index < 0 {
        return Err(AppError::InvalidArgument(format!("{path} 必须是非负整数")));
    }
    Ok(index)
}

pub(in crate::manga::pipeline_prompt::validation) fn validate_manga_beat_ref(
    value: Option<&Value>,
    path: &str,
) -> Result<(i64, i64, i64), AppError> {
    let object = value
        .and_then(Value::as_object)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path} 必须是对象")))?;
    if object.len() != 3
        || !object.contains_key("scene_index")
        || !object.contains_key("shot_index")
        || !object.contains_key("beat_index")
    {
        return Err(AppError::InvalidArgument(format!(
            "{path} 只能包含 scene_index、shot_index 和 beat_index"
        )));
    }
    let scene_index = object
        .get("scene_index")
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path}.scene_index 必须是正整数")))?;
    let beat_index = object
        .get("beat_index")
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path}.beat_index 必须是正整数")))?;
    let shot_index = object
        .get("shot_index")
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path}.shot_index 必须是正整数")))?;
    Ok((scene_index, shot_index, beat_index))
}

pub(in crate::manga::pipeline_prompt::validation) fn validate_manga_beat_refs(
    value: Option<&Value>,
    path: &str,
) -> Result<(), AppError> {
    let refs = value
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path} 必须是数组")))?;
    if refs.is_empty() {
        return Err(AppError::InvalidArgument(format!("{path} 不能为空")));
    }

    let mut seen = HashSet::with_capacity(refs.len());
    for (index, value) in refs.iter().enumerate() {
        let beat_ref = validate_manga_beat_ref(Some(value), &format!("{path}[{index}]"))?;
        if !seen.insert(beat_ref) {
            return Err(AppError::InvalidArgument(format!(
                "{path} 包含重复拍摄节拍引用：scene_index={}, shot_index={}, beat_index={}",
                beat_ref.0, beat_ref.1, beat_ref.2
            )));
        }
    }
    Ok(())
}
