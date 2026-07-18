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
fn source_metadata_matching_and_nested_hash_extraction_require_non_empty_hashes() {
    let stored = json!({ "content_sha256": "same", "content_chars": 10 });
    let matching = json!({ "content_sha256": "same", "content_chars": 99 });
    let changed = json!({ "content_sha256": "changed" });
    assert!(source_metadata_matches(&stored, &matching));
    assert!(!source_metadata_matches(&stored, &changed));

    let nested = json!({ "source": { "content_sha256": "same" } });
    assert_eq!(metadata_source_content_sha256(Some(&nested)), Some("same"));
    assert_eq!(metadata_source_content_sha256(Some(&json!({}))), None);
    assert_eq!(metadata_source_content_sha256(None), None);
}
