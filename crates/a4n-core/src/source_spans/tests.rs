use serde_json::Value;

use crate::models::chapter::chapter;

use super::{source_span_prompt_values, source_spans_for_chapters, source_text_hash};

fn chapter_for_source_span_test(content: &str) -> chapter::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter::Model {
        id: 7,
        project_id: 1,
        outline_id: None,
        chapter_number: 1,
        title: Some("测试".to_string()),
        content: content.to_string(),
        word_count: content.chars().count() as i32,
        status: chapter::STATUS_DRAFT.to_string(),
        summary: None,
        style_tags: None,
        analysis_data: None,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn source_spans_are_contiguous_original_substrings() {
    let chapter = chapter_for_source_span_test(
        "他转身走向厨房，地上有一把军用匕首，刀身上还沾着前一个住户的血迹。",
    );

    let spans = source_spans_for_chapters(std::slice::from_ref(&chapter));
    let texts = spans
        .iter()
        .map(|span| span.text.as_str())
        .collect::<Vec<_>>();

    assert!(texts.contains(&"他转身走向厨房"));
    assert!(texts.contains(&"地上有一把军用匕首"));
    assert!(texts.contains(&"刀身上还沾着前一个住户的血迹"));
    for span in spans {
        let start = chapter
            .content
            .char_indices()
            .nth(span.char_start)
            .map(|(index, _)| index)
            .unwrap_or(chapter.content.len());
        let end = chapter
            .content
            .char_indices()
            .nth(span.char_end)
            .map(|(index, _)| index)
            .unwrap_or(chapter.content.len());
        assert_eq!(chapter.content[start..end], span.text);
    }
}

#[test]
fn source_spans_keep_short_object_names() {
    let chapter = chapter_for_source_span_test("绷带。匕首。");
    let spans = source_spans_for_chapters(&[chapter]);
    let texts = spans
        .iter()
        .map(|span| span.text.as_str())
        .collect::<Vec<_>>();

    assert!(texts.contains(&"绷带"));
    assert!(texts.contains(&"匕首"));
}

#[test]
fn source_span_prompt_values_expose_id_paragraph_and_text() {
    let chapter = chapter_for_source_span_test("林岚走进旧城，墙上刻着隐藏标记。");
    let spans = source_spans_for_chapters(&[chapter]);
    let values = source_span_prompt_values(&spans);
    let first = values
        .first()
        .and_then(Value::as_object)
        .expect("prompt span is an object");

    assert_eq!(
        first.keys().cloned().collect::<Vec<_>>(),
        vec![
            "id".to_string(),
            "paragraph_index".to_string(),
            "text".to_string(),
        ]
    );
    assert!(first.get("id").is_some());
    assert!(first.get("paragraph_index").is_some());
    assert!(first.get("text").is_some());
    assert!(first.get("chapter_id").is_none());
    assert!(first.get("chapter_number").is_none());
    assert!(first.get("char_start").is_none());
    assert!(first.get("char_end").is_none());
    assert!(first.get("long_text").is_none());
}

#[test]
fn source_text_hash_is_sha256_hex() {
    assert_eq!(
        source_text_hash("abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
