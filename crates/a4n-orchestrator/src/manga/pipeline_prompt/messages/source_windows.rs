use std::collections::HashSet;

use serde_json::{Value, json};

use a4n_core::error::AppError;
use a4n_core::source_spans::SourceSpan;
use a4n_core::source_spans::source_span_prompt_value;

pub(super) fn push_unique_source_span_id(
    value: Option<&Value>,
    path: &str,
    ids: &mut Vec<String>,
    seen: &mut HashSet<String>,
) -> Result<(), AppError> {
    let source_span_id = value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::InvalidArgument(format!("{path} 必须是非空字符串")))?;
    if seen.insert(source_span_id.to_string()) {
        ids.push(source_span_id.to_string());
    }
    Ok(())
}

fn push_unique_source_span_ids_from_array(
    value: Option<&Value>,
    path: &str,
    ids: &mut Vec<String>,
    seen: &mut HashSet<String>,
) -> Result<(), AppError> {
    let values = value
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path} 必须是数组")))?;
    for (index, item) in values.iter().enumerate() {
        push_unique_source_span_id(Some(item), &format!("{path}[{index}]"), ids, seen)?;
    }
    Ok(())
}

pub(super) fn event_source_span_windows(
    events_json: &Value,
    source_spans: &[SourceSpan],
) -> Result<Value, AppError> {
    let mut windows = Vec::new();
    let mut available_source_span_ids = Vec::new();
    let mut seen_available = HashSet::new();
    let events = events_json
        .as_array()
        .ok_or_else(|| AppError::InvalidArgument("漫画导演事件投影必须是数组".to_string()))?;

    for (event_index, event) in events.iter().enumerate() {
        let mut ids = Vec::new();
        let mut seen_ids = HashSet::new();
        push_unique_source_span_ids_from_array(
            event.get("source_span_ids"),
            &format!("漫画导演事件[{event_index}].source_span_ids"),
            &mut ids,
            &mut seen_ids,
        )?;
        if ids.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "漫画导演事件[{event_index}].source_span_ids 不能为空"
            )));
        }
        let mut text_parts = Vec::new();
        for id in &ids {
            let span = source_spans
                .iter()
                .find(|span| span.id == *id)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "漫画导演事件[{event_index}] 引用了不存在的 source_span_id: {id}"
                    ))
                })?;
            text_parts.push(span.text.clone());
            if seen_available.insert(span.id.clone()) {
                available_source_span_ids.push(source_span_prompt_value(span));
            }
        }
        let beat_ref = event
            .get("beat_ref")
            .and_then(Value::as_object)
            .ok_or_else(|| AppError::InvalidArgument("漫画导演事件缺少 beat_ref".to_string()))?;
        let scene_index = beat_ref
            .get("scene_index")
            .and_then(Value::as_u64)
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                AppError::InvalidArgument("漫画导演事件 beat_ref.scene_index 无效".to_string())
            })?;
        let beat_index = beat_ref
            .get("beat_index")
            .and_then(Value::as_u64)
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                AppError::InvalidArgument("漫画导演事件 beat_ref.beat_index 无效".to_string())
            })?;
        let shot_index = beat_ref
            .get("shot_index")
            .and_then(Value::as_u64)
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                AppError::InvalidArgument("漫画拍摄事件 beat_ref.shot_index 无效".to_string())
            })?;
        windows.push(json!({
            "window_id": format!("beat_{scene_index}_{shot_index}_{beat_index}"),
            "beat_ref": beat_ref,
            "event": event,
            "source_span_ids": ids,
            "text": text_parts.join("\n")
        }));
    }

    Ok(json!({
        "source_windows": windows,
        "available_source_spans": available_source_span_ids
    }))
}
