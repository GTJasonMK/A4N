use super::*;

#[test]
fn preserves_existing_short_paragraphs() {
    let content = "第一段。\n\n第二段。\n第三段。";

    assert_eq!(
        normalize_generated_chapter_content_layout(content),
        "第一段。\n\n第二段。\n\n第三段。"
    );
}

#[test]
fn splits_single_long_chinese_paragraph_at_sentence_boundaries() {
    let sentence = "陈恪站在廊下，看见雨线从檐角落下，像一排细密的银针。";
    let content = sentence.repeat(20);
    let normalized = normalize_generated_chapter_content_layout(&content);
    let paragraphs: Vec<&str> = normalized.split("\n\n").collect();

    assert!(paragraphs.len() > 1);
    assert!(paragraphs.iter().all(|paragraph| paragraph.ends_with('。')));
    assert_eq!(normalized.replace("\n\n", ""), content);
}

#[test]
fn keeps_closing_quotes_with_sentence() {
    let sentence = "“你真的想清楚了吗？”";
    let content = sentence.repeat(30);
    let normalized = normalize_generated_chapter_content_layout(&content);

    assert!(!normalized.contains("\n\n”"));
    assert_eq!(normalized.replace("\n\n", ""), content);
}
