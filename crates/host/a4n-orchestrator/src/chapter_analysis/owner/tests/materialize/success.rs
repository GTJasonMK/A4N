use super::*;

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
