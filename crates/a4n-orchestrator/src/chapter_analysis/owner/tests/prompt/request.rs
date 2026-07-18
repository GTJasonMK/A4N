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
