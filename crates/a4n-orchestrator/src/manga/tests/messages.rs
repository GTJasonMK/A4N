use super::*;

#[test]
fn build_manga_step_user_message_uses_json_payload() {
    let message = build_manga_step_user_message(
        "manga_speech_selection",
        "选择漫画对白与旁白",
        json!({
            "chapter": {"content": "章节正文"}
        }),
        json!({
            "format": "json_object",
            "required_fields": ["speeches"]
        }),
    )
    .expect("manga step message should build");

    assert!(message.contains(r#""prompt_name": "manga_speech_selection""#));
    assert!(message.contains(r#""input": {"#));
    assert!(message.contains(r#""output_contract": {"#));
    assert!(!message.contains("## 章节内容"));
}

#[test]
fn manga_speech_selection_user_message_uses_event_windows() {
    let source_spans =
        source_spans_for_content(7, 1, "林晓雨低声说，你好。她忽然意识到这不是巧合。");
    let span_id = source_spans
        .iter()
        .find(|span| span.text.contains("你好"))
        .expect("dialogue span")
        .id
        .clone();
    let message = manga_speech_selection_user_message(
        json!({"林晓雨": {"name": "林晓雨"}}),
        json!([{"beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1}, "description": "林晓雨与陌生人对话", "source_span_ids": [span_id]}]),
        &source_spans,
        json!({"character_portraits": []}),
    )
    .expect("speech selection prompt should serialize");

    assert!(message.contains(r#""source_windows": ["#));
    assert!(message.contains(r#""available_source_spans": ["#));
    assert!(!message.contains(r#""content": "林晓雨低声说，你好。她忽然意识到这不是巧合。""#));
    assert!(message.contains(r#""source_span_rule""#));
    assert!(message.contains("input.chapter.available_source_spans"));
    assert!(!message.contains("input.chapter.source_spans"));
    assert!(!message.contains(r#""chapter_id""#));
    assert!(!message.contains(r#""chapter_number""#));
    assert!(!message.contains(r#""char_start""#));
    assert!(!message.contains(r#""char_end""#));
    assert!(!message.contains(r#""long_text""#));
    assert!(message.contains("source_span_id"));
}

#[test]
fn manga_visual_anchor_extraction_user_message_uses_event_windows() {
    let source_spans = source_spans_for_content(7, 1, "林晓雨推开门。陌生人站在雨里。");
    let span_id = source_spans
        .iter()
        .find(|span| span.text.contains("陌生人"))
        .expect("scene span")
        .id
        .clone();
    let message = manga_visual_anchor_extraction_user_message(
        json!([{"beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 1}, "description": "林晓雨推门", "source_span_ids": [span_id]}]),
        &source_spans,
    )
    .expect("visual anchor prompt should serialize");

    assert!(message.contains(r#""source_windows": ["#));
    assert!(message.contains(r#""available_source_spans": ["#));
    assert!(!message.contains(r#""content": "林晓雨推开门。陌生人站在雨里。""#));
    assert!(message.contains(r#""source_span_rule""#));
    assert!(message.contains("input.chapter.available_source_spans"));
    assert!(!message.contains("input.chapter.source_spans"));
    assert!(!message.contains(r#""chapter_id""#));
    assert!(!message.contains(r#""chapter_number""#));
    assert!(!message.contains(r#""char_start""#));
    assert!(!message.contains(r#""char_end""#));
    assert!(!message.contains(r#""long_text""#));
    assert!(message.contains("source_span_ids"));
}

#[test]
fn manga_item_extraction_input_exposes_available_source_span_ids() {
    let step2 = json!({
        "dialogues": [{"source_span_id": "ch1:s1:f1"}],
        "narrations": [{"source_span_id": "ch1:s2:f1"}]
    });
    let step3 = json!({
        "scenes": [{
            "source_span_ids": ["ch1:s2:f1", "ch1:s3:f1"]
        }]
    });

    let input = manga_item_extraction_input(step2.clone(), step3.clone());
    let ids = input["available_source_span_ids"].as_array().unwrap();

    assert_eq!(
        ids,
        &vec![json!("ch1:s1:f1"), json!("ch1:s2:f1"), json!("ch1:s3:f1")]
    );
    assert_eq!(
        manga_item_available_source_span_ids(&step2, &step3),
        vec!["ch1:s1:f1", "ch1:s2:f1", "ch1:s3:f1"]
    );
}
