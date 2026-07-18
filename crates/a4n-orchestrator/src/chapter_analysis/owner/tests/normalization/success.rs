#[test]
fn normalize_analysis_keeps_valid_payload_and_adds_meta() {
    let ch = sample_chapter();
    let normalized = normalize_analysis(sample_analysis(), &ch).unwrap();

    assert_eq!(normalized["metadata"]["characters"][0], "林岚");
    assert_eq!(normalized["_meta"]["project_id"], 22);
    assert_eq!(normalized["_meta"]["chapter_id"], 11);
    assert_eq!(
        normalized["_meta"]["content_sha256"],
        source_text_hash(&ch.content)
    );
}

#[test]
fn validate_saved_analysis_accepts_normalized_payload_with_meta() {
    let normalized = normalize_analysis(sample_analysis(), &sample_chapter()).unwrap();

    a4n_core::chapter_analysis::validate_saved_chapter_analysis(&normalized)
        .expect("normalized saved analysis should be valid");
}
