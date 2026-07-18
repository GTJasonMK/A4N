#[test]
fn validate_unique_deletion_candidates_rejects_duplicate_in_same_chapter() {
    let candidates = vec![
        json!({
            "chapter_number": 3,
            "category": "explicit",
            "key": "护身玉佩",
            "reason": "玉佩损毁",
            "evidence_anchor_id": 31
        }),
        json!({
            "chapter_number": 3,
            "category": "explicit",
            "key": "护身玉佩",
            "reason": "玉佩彻底损毁",
            "evidence_anchor_id": 32
        }),
    ];

    let err = validate_unique_deletion_candidates(&candidates)
        .expect_err("duplicate deletion candidate in one chapter must fail");

    assert!(err.contains("删除候选重复"));
    assert!(err.contains("第 3 章 explicit.护身玉佩"));
}

#[test]
fn validate_unique_deletion_candidates_allows_consecutive_chapter_marks() {
    let candidates = vec![
        json!({
            "chapter_number": 3,
            "category": "explicit",
            "key": "护身玉佩",
            "reason": "玉佩裂开",
            "evidence_anchor_id": 31
        }),
        json!({
            "chapter_number": 4,
            "category": "explicit",
            "key": "护身玉佩",
            "reason": "玉佩仍不可用",
            "evidence_anchor_id": 41
        }),
    ];

    validate_unique_deletion_candidates(&candidates)
        .expect("same attribute across chapters should count as deletion-protection timeline");
}
