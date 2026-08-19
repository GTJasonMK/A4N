use super::*;

#[test]
fn source_metadata_contains_hash_and_character_count() {
    let metadata = source_metadata(ChapterSourceContext {
        content: "正文内容",
    });

    assert_eq!(metadata["content_sha256"], source_text_hash("正文内容"));
    assert_eq!(metadata["content_chars"], 4);
}

#[test]
fn source_metadata_flags_extend_without_replacing_base_fields() {
    let metadata = source_metadata_with_flags(
        ChapterSourceContext { content: "正文" },
        [("is_summary", true), ("is_outline", false)],
    );

    assert_eq!(metadata["content_chars"], 2);
    assert_eq!(metadata["is_summary"], true);
    assert_eq!(metadata["is_outline"], false);
}

#[test]
fn nested_hash_extraction_requires_non_empty_hashes() {
    let nested = json!({ "source": { "content_sha256": "same" } });
    assert_eq!(metadata_source_content_sha256(Some(&nested)), Some("same"));
    assert_eq!(metadata_source_content_sha256(Some(&json!({}))), None);
    assert_eq!(metadata_source_content_sha256(None), None);
}
