use super::*;
use std::collections::{HashMap, HashSet};

#[test]
fn rag_results_reject_unknown_fields() {
    let error = serde_json::from_value::<SearchResult>(serde_json::json!({
        "id": "record-1",
        "content": "content",
        "score": 1.0,
        "vector_score": 1.0,
        "bm25_score": null,
        "metadata": {},
        "legacy_rank": 1
    }))
    .expect_err("RAG search result must reject unknown fields");
    assert!(error.to_string().contains("legacy_rank"));
}

#[test]
fn search_filter_applies_equals_data_type_and_bounded_chapter() {
    let filter = SearchFilter {
        equals: Some(HashMap::from([(
            "project_id".to_string(),
            serde_json::json!(7),
        )])),
        chapter_range: Some((1, 10)),
        data_types: Some(vec!["chapter".to_string()]),
    };

    assert!(metadata_matches_search_filter(
        &serde_json::json!({
            "project_id": 7,
            "data_type": "chapter",
            "chapter_number": 6
        }),
        &filter
    ));
    assert!(!metadata_matches_search_filter(
        &serde_json::json!({
            "project_id": 8,
            "data_type": "chapter",
            "chapter_number": 6
        }),
        &filter
    ));
    assert!(!metadata_matches_search_filter(
        &serde_json::json!({
            "project_id": 7,
            "data_type": "profile",
            "chapter_number": 6
        }),
        &filter
    ));
    assert!(!metadata_matches_search_filter(
        &serde_json::json!({
            "project_id": 7,
            "data_type": "chapter",
            "chapter_number": u64::from(u32::MAX) + 1
        }),
        &filter
    ));
}

#[test]
fn expected_id_completeness_classifies_vector_and_bm25_gaps() {
    let expected = vec![
        "record-c".to_string(),
        "record-a".to_string(),
        "record-b".to_string(),
    ];
    let vector = HashSet::from(["record-a".to_string(), "record-c".to_string()]);
    let bm25 = HashSet::from(["record-a".to_string(), "record-b".to_string()]);

    let report = CompletenessReport::by_expected_ids("project-7", &expected, &vector, Some(&bm25));

    assert!(!report.is_complete);
    assert_eq!(report.missing_vector_ids, vec!["record-b".to_string()]);
    assert_eq!(report.missing_bm25_ids, vec!["record-c".to_string()]);
    assert_eq!(
        report.missing_ids,
        vec!["record-b".to_string(), "record-c".to_string()]
    );
}
