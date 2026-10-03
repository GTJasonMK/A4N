use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};

use crate::source_markers::SourceMarkerMap;
use a4n_domain::media_authoring::AdaptationBeatRef;
use a4n_domain::source_spans::SourceSpan;
use a4n_error::AppError;

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
    source_markers: &SourceMarkerMap,
) -> Result<Value, AppError> {
    let mut windows = Vec::new();
    let mut available_source_span_ids = Vec::new();
    let mut seen_available = HashSet::new();
    let events = events_json
        .as_array()
        .ok_or_else(|| AppError::InvalidArgument("漫画改编事件投影必须是数组".to_string()))?;
    let source_spans_by_id = source_spans.iter().fold(
        HashMap::with_capacity(source_spans.len()),
        |mut spans_by_id, span| {
            spans_by_id.entry(span.id.as_str()).or_insert(span);
            spans_by_id
        },
    );

    for (event_index, event) in events.iter().enumerate() {
        let mut ids = Vec::new();
        let mut seen_ids = HashSet::new();
        push_unique_source_span_ids_from_array(
            event.get("source_span_ids"),
            &format!("漫画改编事件[{event_index}].source_span_ids"),
            &mut ids,
            &mut seen_ids,
        )?;
        if ids.is_empty() && event.get("origin").and_then(Value::as_str) == Some("source") {
            return Err(AppError::InvalidArgument(format!(
                "漫画改编事件[{event_index}].source_span_ids 不能为空"
            )));
        }
        let mut text_parts = Vec::new();
        for id in &ids {
            let span = source_spans_by_id
                .get(id.as_str())
                .copied()
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "漫画改编事件[{event_index}] 引用了不存在的 source_span_id: {id}"
                    ))
                })?;
            text_parts.push(format!(
                "{} {}",
                source_markers.marker_for(&span.id)?,
                span.text
            ));
            if seen_available.insert(span.id.clone()) {
                available_source_span_ids.push(span.id.clone());
            }
        }
        let raw_ref = event
            .get("beat_ref")
            .ok_or_else(|| AppError::InvalidArgument("漫画改编事件缺少 beat_ref".to_string()))?;
        let beat_ref: AdaptationBeatRef = serde_json::from_value(raw_ref.clone())?;
        beat_ref.validate().map_err(AppError::InvalidArgument)?;
        windows.push(json!({
            "window_id": serde_json::to_string(&beat_ref)?,
            "beat_ref": beat_ref,
            "event": event,
            "source_span_ids": ids,
            "text": text_parts.join("\n")
        }));
    }

    Ok(json!({
        "source_windows": windows,
        "available_source_span_ids": available_source_span_ids
    }))
}
