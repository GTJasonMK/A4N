use std::collections::HashSet;

use a4n_core::error::AppError;
use serde_json::Value;

use super::super::materialize::analysis_materialize_error;

mod required;
use required::validate_required_fields;
mod root;
use root::validate_root_fields;

pub fn validate_chapter_foreshadowing_events(value: &Value, facts: &Value) -> Result<(), AppError> {
    validate_root_fields(value)?;
    validate_required_fields(value)?;

    let event_span_ids = facts
        .get("event_candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|candidate| candidate.get("source_span_ids").and_then(Value::as_array))
        .flatten()
        .filter_map(Value::as_str)
        .collect::<HashSet<_>>();
    let events = value
        .get("key_events")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            analysis_materialize_error("chapter_foreshadowing_events.key_events 必须是数组")
        })?;
    for (event_index, event) in events.iter().enumerate() {
        let ids = event
            .get("source_span_ids")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                analysis_materialize_error(format!(
                    "chapter_foreshadowing_events.key_events[{event_index}].source_span_ids 必须是数组"
                ))
            })?;
        for (id_index, id) in ids.iter().enumerate() {
            let id = id.as_str().map(str::trim).filter(|id| !id.is_empty()).ok_or_else(|| {
                analysis_materialize_error(format!(
                    "chapter_foreshadowing_events.key_events[{event_index}].source_span_ids[{id_index}] 必须是非空字符串"
                ))
            })?;
            if !event_span_ids.contains(id) {
                return Err(analysis_materialize_error(format!(
                    "chapter_foreshadowing_events.key_events[{event_index}].source_span_ids[{id_index}] 不在事实候选中: {id}"
                )));
            }
        }
    }

    let foreshadowing_span_ids = facts
        .get("foreshadowing_candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|candidate| {
            candidate
                .get("original_text_span_id")
                .and_then(Value::as_str)
        })
        .collect::<HashSet<_>>();
    let foreshadowing = value
        .get("foreshadowing")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            analysis_materialize_error("chapter_foreshadowing_events.foreshadowing 必须是对象")
        })?;
    for key in ["planted", "resolved"] {
        let items = foreshadowing
            .get(key)
            .and_then(Value::as_array)
            .ok_or_else(|| {
                analysis_materialize_error(format!(
                    "chapter_foreshadowing_events.foreshadowing.{key} 必须是数组"
                ))
            })?;
        for (index, item) in items.iter().enumerate() {
            let id = item
                .get("original_text_span_id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .ok_or_else(|| {
                    analysis_materialize_error(format!(
                        "chapter_foreshadowing_events.foreshadowing.{key}[{index}].original_text_span_id 必须是非空字符串"
                    ))
                })?;
            if !foreshadowing_span_ids.contains(id) {
                return Err(analysis_materialize_error(format!(
                    "chapter_foreshadowing_events.foreshadowing.{key}[{index}].original_text_span_id 不在事实候选中: {id}"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::validate_chapter_foreshadowing_events;

    #[test]
    fn validates_selected_source_spans_against_fact_candidates() {
        let facts = json!({
            "event_candidates": [{"source_span_ids": ["ch1:s2:f1"]}],
            "foreshadowing_candidates": [{"original_text_span_id": "ch1:s6:f3"}]
        });
        let mut output = json!({
            "foreshadowing": {
                "planted": [{"original_text_span_id": "ch1:s6:f3"}],
                "resolved": [],
                "tensions": []
            },
            "key_events": [{"source_span_ids": ["ch1:s2:f1"]}]
        });

        validate_chapter_foreshadowing_events(&output, &facts)
            .expect("candidate source spans should pass");

        output["key_events"][0]["source_span_ids"] = json!(["ch1:s4:f1"]);
        let error = validate_chapter_foreshadowing_events(&output, &facts)
            .expect_err("prompt example source span must be rejected");
        assert!(error.to_string().contains("不在事实候选中: ch1:s4:f1"));
    }
}
