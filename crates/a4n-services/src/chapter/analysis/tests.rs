use super::normalization::{content_sha256, normalize_analysis, validate_saved_analysis};
use super::prompt::{build_chapter_analysis_stage_user_message, required_chapter_title};
use super::*;
use crate::chapter::source_spans::{source_span_prompt_values, source_spans_for_chapters};
use a4n_core::models::chapter::chapter;
use chrono::NaiveDate;
use serde_json::json;

fn sample_chapter() -> chapter::Model {
    let now = NaiveDate::from_ymd_opt(2026, 5, 26)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    chapter::Model {
        id: 11,
        project_id: 22,
        outline_id: None,
        chapter_number: 3,
        title: Some("第三章".into()),
        content: "正文".into(),
        word_count: 2,
        status: "completed".into(),
        summary: None,
        style_tags: None,
        analysis_data: None,
        created_at: now,
        updated_at: now,
    }
}

fn sample_analysis() -> Value {
    json!({
        "metadata": {
            "characters": ["林岚"],
            "locations": ["旧城"],
            "items": [],
            "tags": ["转折"],
            "tone": "紧张",
            "timeline_marker": null
        },
        "summaries": {
            "compressed": "林岚在旧城发现新的线索，并意识到此前事件并非偶然。",
            "one_line": "林岚发现旧城线索。",
            "keywords": ["旧城", "线索", "转折"]
        },
        "character_states": {
            "林岚": {
                "location": "旧城",
                "status": "追查线索",
                "changes": ["发现隐藏标记"],
                "emotional_state": "警觉"
            }
        },
        "foreshadowing": {
            "planted": [],
            "resolved": [],
            "tensions": []
        },
        "key_events": [
            {
                "type": "discovery",
                "description": "发现隐藏标记",
                "importance": "high",
                "involved_characters": ["林岚"]
            }
        ]
    })
}

#[test]
fn normalize_analysis_keeps_valid_payload_and_adds_meta() {
    let ch = sample_chapter();
    let normalized = normalize_analysis(sample_analysis(), &ch).unwrap();

    assert_eq!(normalized["metadata"]["characters"][0], "林岚");
    assert_eq!(normalized["_meta"]["project_id"], 22);
    assert_eq!(normalized["_meta"]["chapter_id"], 11);
    assert_eq!(
        normalized["_meta"]["content_sha256"],
        content_sha256(&ch.content)
    );
}

#[test]
fn validate_saved_analysis_accepts_normalized_payload_with_meta() {
    let normalized = normalize_analysis(sample_analysis(), &sample_chapter()).unwrap();

    validate_saved_analysis(&normalized).expect("normalized saved analysis should be valid");
}

#[test]
fn validate_saved_analysis_rejects_missing_required_field() {
    let mut payload = normalize_analysis(sample_analysis(), &sample_chapter()).unwrap();
    payload.as_object_mut().unwrap().remove("summaries");

    let err = validate_saved_analysis(&payload).unwrap_err();

    assert!(matches!(err, AppError::InvalidArgument(message) if message.contains("summaries")));
}

#[test]
fn chapter_analysis_requires_available_content() {
    let mut ch = sample_chapter();
    ch.content = "   ".to_string();

    let err = ensure_chapter_has_analysis_content(&ch).unwrap_err();

    assert!(matches!(err, AppError::InvalidArgument(message) if message.contains("章节正文为空")));
}

#[test]
fn normalize_analysis_rejects_non_object_payload() {
    let err = normalize_analysis(json!([]), &sample_chapter()).unwrap_err();

    assert!(matches!(err, AppError::InvalidArgument(message) if message.contains("JSON 对象")));
}

#[test]
fn normalize_analysis_rejects_missing_required_field() {
    let mut payload = sample_analysis();
    payload.as_object_mut().unwrap().remove("character_states");

    let err = normalize_analysis(payload, &sample_chapter()).unwrap_err();

    assert!(
        matches!(err, AppError::InvalidArgument(message) if message.contains("character_states"))
    );
}

#[test]
fn normalize_analysis_rejects_unknown_top_level_field() {
    let mut payload = sample_analysis();
    payload["legacy_summary"] = json!("旧字段");

    let err = normalize_analysis(payload, &sample_chapter()).unwrap_err();

    assert!(
        matches!(err, AppError::InvalidArgument(message) if message.contains("未知字段 legacy_summary"))
    );
}

#[test]
fn normalize_analysis_rejects_unknown_nested_field() {
    let mut payload = sample_analysis();
    payload["character_states"]["林岚"]["emotion"] = json!("旧情绪字段");

    let err = normalize_analysis(payload, &sample_chapter()).unwrap_err();

    assert!(
        matches!(err, AppError::InvalidArgument(message) if message.contains("未知字段 emotion"))
    );
}

#[test]
fn normalize_analysis_rejects_default_placeholder_string() {
    let mut payload = sample_analysis();
    payload["metadata"]["tone"] = json!("");

    let err = normalize_analysis(payload, &sample_chapter()).unwrap_err();

    assert!(matches!(err, AppError::InvalidArgument(message) if message.contains("非空字符串")));
}

#[test]
fn parse_chapter_analysis_stage_json_rejects_fenced_json() {
    let err = parse_chapter_analysis_stage_json("```json\n{}\n```", "chapter_fact_extraction")
        .unwrap_err();

    assert!(
        matches!(err, AppError::InvalidArgument(message) if message.contains("严格 JSON 对象"))
    );
}

#[test]
fn chapter_analysis_stage_user_message_preserves_full_content() {
    let mut content = "甲".repeat(12_050);
    content.push_str("尾部关键线索");

    let message = build_chapter_analysis_stage_user_message(
        "chapter_fact_extraction",
        "抽取事实",
        json!({
            "chapter_title": "第一章",
            "content": content,
            "source_spans": []
        }),
        json!({
            "format": "json_object",
            "required_fields": ["characters"]
        }),
    )
    .expect("message");

    assert!(message.contains("尾部关键线索"));
}

#[test]
fn chapter_fact_prompt_input_omits_system_derived_source_span_fields() {
    let mut ch = sample_chapter();
    ch.content = "林岚走进旧城，墙上刻着隐藏标记。".to_string();
    let spans = source_spans_for_chapters(std::slice::from_ref(&ch));
    let message = build_chapter_analysis_stage_user_message(
        "chapter_fact_extraction",
        "抽取事实",
        json!({
            "chapter_title": "第三章",
            "content": ch.content,
            "source_spans": source_span_prompt_values(&spans)
        }),
        json!({
            "format": "json_object",
            "required_fields": ["characters"]
        }),
    )
    .expect("message");

    assert!(!message.contains(r#""chapter_id""#));
    assert!(!message.contains(r#""chapter_number""#));
    assert!(!message.contains(r#""char_start""#));
    assert!(!message.contains(r#""char_end""#));
    assert!(!message.contains(r#""long_text""#));
    assert!(message.contains(r#""id": "ch3:s1:f1""#));
    assert!(message.contains(r#""text": "林岚走进旧城""#));
}

#[test]
fn materialize_analysis_uses_source_span_ids_for_quotes() {
    let mut ch = sample_chapter();
    ch.content = "林岚走进旧城，墙上刻着隐藏标记。".to_string();
    let spans = source_spans_for_chapters(std::slice::from_ref(&ch));
    let prompt_values = source_span_prompt_values(&spans);
    let marker_span = prompt_values
        .iter()
        .find(|span| span["text"] == "墙上刻着隐藏标记")
        .and_then(|span| span["id"].as_str())
        .expect("marker span");
    let raw = json!({
        "metadata": {
            "characters": [{"name": "林岚", "source_span_ids": [marker_span]}],
            "locations": [{"name": "旧城", "source_span_ids": [marker_span]}],
            "items": [],
            "tags": ["发现"],
            "tone": "紧张",
            "timeline_marker": null
        },
        "summaries": {
            "compressed": "林岚在旧城发现墙上的隐藏标记，意识到这里存在新的线索。",
            "one_line": "林岚发现隐藏标记。",
            "keywords": ["旧城", "隐藏标记", "线索"]
        },
        "character_states": {
            "林岚": {
                "location": "旧城",
                "status": "追查线索",
                "changes": ["发现隐藏标记"],
                "emotional_state": "警觉"
            }
        },
        "foreshadowing": {
            "planted": [{
                "description": "墙上的隐藏标记指向后续线索",
                "original_text_span_id": marker_span,
                "category": "plot_twist",
                "priority": "medium",
                "related_entities": ["林岚"]
            }],
            "resolved": [],
            "tensions": []
        },
        "key_events": [{
            "type": "discovery",
            "description": "发现隐藏标记",
            "importance": "high",
            "involved_characters": ["林岚"],
            "source_span_ids": [marker_span]
        }]
    });

    let materialized = materialize_analysis_source_spans(raw, &spans).unwrap();

    assert_eq!(materialized["metadata"]["characters"][0], "林岚");
    assert_eq!(
        materialized["foreshadowing"]["planted"][0]["original_text"],
        "墙上刻着隐藏标记"
    );
    assert!(
        materialized["key_events"][0]
            .get("source_span_ids")
            .is_none()
    );
}

#[test]
fn materialize_analysis_allows_freeform_metadata_tags_and_tone() {
    let mut ch = sample_chapter();
    ch.content = "林岚在旧城醒来，发现自己回到了三年前。".to_string();
    let spans = source_spans_for_chapters(std::slice::from_ref(&ch));
    let marker_span = source_span_prompt_values(&spans)
        .iter()
        .find(|span| span["text"] == "发现自己回到了三年前")
        .and_then(|span| span["id"].as_str())
        .expect("marker span")
        .to_string();
    let raw = json!({
        "metadata": {
            "characters": [{"name": "林岚", "source_span_ids": [marker_span]}],
            "locations": [{"name": "旧城", "source_span_ids": [marker_span]}],
            "items": [],
            "tags": [" 重生 ", "转折", "重生", ""],
            "tone": " 紧张且狠厉 ",
            "timeline_marker": " 三年前 "
        },
        "summaries": {
            "compressed": "林岚在旧城醒来后，发现自己回到了三年前，意识到命运出现重大转折。",
            "one_line": "林岚重生回到三年前。",
            "keywords": ["旧城", "重生", "三年前"]
        },
        "character_states": {
            "林岚": {
                "location": "旧城",
                "status": "刚刚醒来",
                "changes": ["回到三年前"],
                "emotional_state": "震惊"
            }
        },
        "foreshadowing": {
            "planted": [],
            "resolved": [],
            "tensions": []
        },
        "key_events": [{
            "type": "revelation",
            "description": "林岚发现自己回到三年前",
            "importance": "high",
            "involved_characters": ["林岚"],
            "source_span_ids": [marker_span]
        }]
    });

    let materialized = materialize_analysis_source_spans(raw, &spans).unwrap();

    assert_eq!(materialized["metadata"]["tags"], json!(["重生", "转折"]));
    assert_eq!(materialized["metadata"]["tone"], "紧张且狠厉");
    assert_eq!(materialized["metadata"]["timeline_marker"], "三年前");
}

#[test]
fn materialize_analysis_rejects_legacy_foreshadowing_evidence() {
    let mut ch = sample_chapter();
    ch.content = "林岚走进旧城，墙上刻着隐藏标记。".to_string();
    let spans = source_spans_for_chapters(std::slice::from_ref(&ch));
    let marker_span = source_span_prompt_values(&spans)
        .iter()
        .find(|span| span["text"] == "墙上刻着隐藏标记")
        .and_then(|span| span["id"].as_str())
        .expect("marker span")
        .to_string();
    let raw = json!({
        "metadata": {
            "characters": [{"name": "林岚", "source_span_ids": [marker_span]}],
            "locations": [{"name": "旧城", "source_span_ids": [marker_span]}],
            "items": [],
            "tags": ["发现"],
            "tone": "紧张",
            "timeline_marker": null
        },
        "summaries": {
            "compressed": "林岚在旧城发现墙上的隐藏标记。",
            "one_line": "林岚发现隐藏标记。",
            "keywords": ["旧城", "隐藏标记"]
        },
        "character_states": {},
        "foreshadowing": {
            "planted": [{
                "description": "墙上的隐藏标记指向后续线索",
                "original_text_span_id": marker_span,
                "original_text": "墙上刻着隐藏标记",
                "category": "plot_twist",
                "priority": "medium",
                "related_entities": ["林岚"]
            }],
            "resolved": [],
            "tensions": []
        },
        "key_events": []
    });

    let err = materialize_analysis_source_spans(raw, &spans).unwrap_err();

    assert!(
        matches!(err, AppError::InvalidArgument(message) if message.contains("未知字段 original_text"))
    );
}

#[test]
fn materialize_analysis_rejects_unknown_metadata_entity_field() {
    let mut ch = sample_chapter();
    ch.content = "林岚走进旧城，墙上刻着隐藏标记。".to_string();
    let spans = source_spans_for_chapters(std::slice::from_ref(&ch));
    let marker_span = source_span_prompt_values(&spans)
        .iter()
        .find(|span| span["text"] == "墙上刻着隐藏标记")
        .and_then(|span| span["id"].as_str())
        .expect("marker span")
        .to_string();
    let raw = json!({
        "metadata": {
            "characters": [{"name": "林岚", "source_span_ids": [marker_span], "role": "主角"}],
            "locations": [],
            "items": [],
            "tags": ["发现"],
            "tone": "紧张",
            "timeline_marker": null
        },
        "summaries": {
            "compressed": "林岚在旧城发现墙上的隐藏标记。",
            "one_line": "林岚发现隐藏标记。",
            "keywords": ["旧城", "隐藏标记"]
        },
        "character_states": {},
        "foreshadowing": {
            "planted": [],
            "resolved": [],
            "tensions": []
        },
        "key_events": []
    });

    let err = materialize_analysis_source_spans(raw, &spans).unwrap_err();

    assert!(matches!(err, AppError::InvalidArgument(message) if message.contains("未知字段 role")));
}

#[test]
fn required_chapter_title_rejects_missing_title() {
    let mut ch = sample_chapter();
    ch.title = Some("  ".into());

    let err = required_chapter_title(&ch).unwrap_err();

    assert!(matches!(err, AppError::InvalidArgument(message) if message.contains("缺少章节标题")));
}

#[test]
fn character_state_snapshots_require_changes_array() {
    let mut analysis = sample_analysis();
    analysis["character_states"]["林岚"]
        .as_object_mut()
        .unwrap()
        .remove("changes");

    assert!(character_state_snapshots(&analysis).is_empty());
}

#[test]
fn foreshadowing_entries_do_not_promote_tensions_to_pending_entries() {
    let mut analysis = sample_analysis();
    analysis["foreshadowing"]["tensions"] = json!(["敌人去向未明"]);

    assert!(foreshadowing_entries(&analysis).is_empty());
}

#[test]
fn priority_to_importance_rejects_missing_or_unknown_priority() {
    assert!(matches!(
        priority_to_importance(None),
        Err(AppError::InvalidArgument(message)) if message.contains("优先级缺失")
    ));
    assert!(matches!(
        priority_to_importance(Some("critical")),
        Err(AppError::InvalidArgument(message)) if message.contains("优先级非法")
    ));
    assert_eq!(priority_to_importance(Some("high")).unwrap(), 5);
    assert_eq!(priority_to_importance(Some("中")).unwrap(), 3);
    assert_eq!(priority_to_importance(Some("低")).unwrap(), 1);
}
