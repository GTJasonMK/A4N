use super::*;

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
