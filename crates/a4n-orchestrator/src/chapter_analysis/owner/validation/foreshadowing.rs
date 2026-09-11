use std::collections::HashSet;

use a4n_error::AppError;
use serde_json::Value;

use super::super::materialize::analysis_materialize_error;

mod required;
use required::validate_required_fields;
mod root;
use root::validate_root_fields;

pub fn validate_chapter_foreshadowing_events(
    value: &Value,
    facts: &Value,
    pending_foreshadowing: &Value,
) -> Result<(), AppError> {
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
    let planted = foreshadowing
        .get("planted")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            analysis_materialize_error(
                "chapter_foreshadowing_events.foreshadowing.planted 必须是数组",
            )
        })?;
    for (index, item) in planted.iter().enumerate() {
        let id = item
            .get("original_text_span_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| {
                analysis_materialize_error(format!(
                    "chapter_foreshadowing_events.foreshadowing.planted[{index}].original_text_span_id 必须是非空字符串"
                ))
            })?;
        if !foreshadowing_span_ids.contains(id) {
            return Err(analysis_materialize_error(format!(
                "chapter_foreshadowing_events.foreshadowing.planted[{index}].original_text_span_id 不在事实候选中: {id}"
            )));
        }
    }

    let allowed_resolution_ids = pending_foreshadowing
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("id").and_then(Value::as_str))
        .collect::<HashSet<_>>();
    let resolved = foreshadowing
        .get("resolved")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            analysis_materialize_error(
                "chapter_foreshadowing_events.foreshadowing.resolved 必须是数组",
            )
        })?;
    let mut seen_resolution_ids = HashSet::new();
    for (index, item) in resolved.iter().enumerate() {
        let id = item
            .get("id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| {
                analysis_materialize_error(format!(
                    "chapter_foreshadowing_events.foreshadowing.resolved[{index}].id 必须是非空字符串"
                ))
            })?;
        if !allowed_resolution_ids.contains(id) {
            return Err(analysis_materialize_error(format!(
                "chapter_foreshadowing_events.foreshadowing.resolved[{index}].id 不在待回收伏笔中: {id}"
            )));
        }
        if !seen_resolution_ids.insert(id) {
            return Err(analysis_materialize_error(format!(
                "chapter_foreshadowing_events.foreshadowing.resolved[{index}].id 重复: {id}"
            )));
        }
        item.get("resolution")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|resolution| !resolution.is_empty())
            .ok_or_else(|| {
                analysis_materialize_error(format!(
                    "chapter_foreshadowing_events.foreshadowing.resolved[{index}].resolution 必须是非空字符串"
                ))
            })?;
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
        let pending = json!([{"id": "7"}]);
        let mut output = json!({
            "foreshadowing": {
                "planted": [{"original_text_span_id": "ch1:s6:f3"}],
                "resolved": [],
                "tensions": []
            },
            "key_events": [{"source_span_ids": ["ch1:s2:f1"]}]
        });

        validate_chapter_foreshadowing_events(&output, &facts, &pending)
            .expect("candidate source spans should pass");

        output["key_events"][0]["source_span_ids"] = json!(["ch1:s4:f1"]);
        let error = validate_chapter_foreshadowing_events(&output, &facts, &pending)
            .expect_err("prompt example source span must be rejected");
        assert!(error.to_string().contains("不在事实候选中: ch1:s4:f1"));
    }

    #[test]
    fn resolved_ids_must_come_from_request_context_and_be_unique() {
        let facts = json!({
            "event_candidates": [],
            "foreshadowing_candidates": []
        });
        let pending = json!([{"id": "7"}, {"id": "9"}]);
        let mut output = json!({
            "foreshadowing": {
                "planted": [],
                "resolved": [{"id": "7", "resolution": "脚步声来自守门人"}],
                "tensions": []
            },
            "key_events": []
        });

        validate_chapter_foreshadowing_events(&output, &facts, &pending)
            .expect("provided id should pass");

        output["foreshadowing"]["resolved"][0]["id"] = json!("8");
        assert!(
            validate_chapter_foreshadowing_events(&output, &facts, &pending)
                .unwrap_err()
                .to_string()
                .contains("不在待回收伏笔中: 8")
        );

        output["foreshadowing"]["resolved"] = json!([
            {"id": "7", "resolution": "第一次"},
            {"id": "7", "resolution": "第二次"}
        ]);
        assert!(
            validate_chapter_foreshadowing_events(&output, &facts, &pending)
                .unwrap_err()
                .to_string()
                .contains("id 重复: 7")
        );
    }
}
