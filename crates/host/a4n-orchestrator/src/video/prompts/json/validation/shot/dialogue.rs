use a4n_error::AppError;
use serde_json::{Map, Value};

use super::super::super::VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS;
use super::super::validate_structured_value;

#[derive(Debug, Clone, Copy)]
pub(super) enum DialogueTracePolicy {
    ModelOutput,
    InternalNormalized,
}

pub(super) fn validate_dialogue_beats(
    value: &Value,
    allowed_source_span_ids: &[String],
    policy: DialogueTracePolicy,
    director_source: bool,
) -> Result<Value, AppError> {
    let beats = value.as_array().ok_or_else(|| {
        AppError::InvalidArgument("视频提示词 dialogue_cues 必须是数组".to_string())
    })?;

    if beats.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }
    if allowed_source_span_ids.is_empty() && !director_source {
        return Err(AppError::InvalidArgument(
            "视频提示词 dialogue_cues 无法补齐 source_span_ids：当前片段 source_span_ids 为空"
                .to_string(),
        ));
    }

    let derived_source_span_ids = Value::Array(
        allowed_source_span_ids
            .iter()
            .map(|id| Value::String(id.clone()))
            .collect(),
    );
    let mut normalized = Vec::with_capacity(beats.len());
    for (index, beat) in beats.iter().enumerate() {
        let object = beat.as_object().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频提示词 dialogue_cues[{}] 必须是对象",
                index + 1
            ))
        })?;
        validate_dialogue_trace_fields(object, index, allowed_source_span_ids, policy)?;
        for key in object.keys() {
            if !(VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS.contains(&key.as_str())
                || matches!(policy, DialogueTracePolicy::InternalNormalized)
                    && key == "source_span_ids")
            {
                return Err(AppError::InvalidArgument(format!(
                    "视频提示词 dialogue_cues[{}].{} 不是允许字段；dialogue_cues 每条只能包含 {}",
                    index + 1,
                    key,
                    VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS.join("、")
                )));
            }
        }
        for key in VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS {
            validate_structured_value(
                object.get(*key).unwrap_or(&Value::Null),
                &format!("视频提示词 dialogue_cues[{}].{}", index + 1, key),
            )?;
        }
        let mut normalized_object = object.clone();
        normalized_object.insert(
            "source_span_ids".to_string(),
            derived_source_span_ids.clone(),
        );
        normalized.push(Value::Object(normalized_object));
    }
    Ok(Value::Array(normalized))
}

fn validate_dialogue_trace_fields(
    object: &Map<String, Value>,
    index: usize,
    allowed_source_span_ids: &[String],
    policy: DialogueTracePolicy,
) -> Result<(), AppError> {
    if object.contains_key("source_span_id") {
        return Err(AppError::InvalidArgument(format!(
            "视频提示词 dialogue_cues[{}] 不要输出 source_span_id/source_span_ids；后端会根据当前片段 source_span_ids 补齐内部溯源",
            index + 1
        )));
    }

    match policy {
        DialogueTracePolicy::ModelOutput => {
            if object.contains_key("source_span_ids") {
                return Err(AppError::InvalidArgument(format!(
                    "视频提示词 dialogue_cues[{}] 不要输出 source_span_id/source_span_ids；后端会根据当前片段 source_span_ids 补齐内部溯源",
                    index + 1
                )));
            }
        }
        DialogueTracePolicy::InternalNormalized => {
            let stored_ids = object
                .get("source_span_ids")
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "已保存视频提示词 dialogue_beats[{}] 缺少后端补齐的 source_span_ids",
                        index + 1
                    ))
                })?
                .as_array()
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "已保存视频提示词 dialogue_beats[{}].source_span_ids 必须是数组",
                        index + 1
                    ))
                })?;
            let stored_ids = stored_ids
                .iter()
                .map(|value| {
                    value.as_str().map(str::to_string).ok_or_else(|| {
                        AppError::InvalidArgument(format!(
                            "已保存视频提示词 dialogue_beats[{}].source_span_ids 只能包含字符串",
                            index + 1
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            if stored_ids != allowed_source_span_ids {
                return Err(AppError::InvalidArgument(format!(
                    "已保存视频提示词 dialogue_beats[{}].source_span_ids 与当前片段溯源不一致，请重新生成该视频提示词",
                    index + 1
                )));
            }
        }
    }
    Ok(())
}
