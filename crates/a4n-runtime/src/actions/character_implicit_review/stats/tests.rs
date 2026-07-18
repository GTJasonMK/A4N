use super::*;
use crate::actions::character_implicit_review::types::BehaviorForReview;
use serde_json::json;

fn behavior(id: i64, chapter_number: i32, classification: serde_json::Value) -> BehaviorForReview {
    BehaviorForReview {
        behavior_id: id,
        chapter_number,
        behavior_type: "decision".to_string(),
        behavior_description: format!("第{chapter_number}章行为"),
        original_text: "原文证据".to_string(),
        behavior_tags: json!(["标签"]),
        classification_results: classification,
        created_at: None,
    }
}

#[test]
fn review_params_reject_zero_or_out_of_range_values() {
    let threshold_err =
        validate_implicit_review_params(0, 10, 10).expect_err("zero threshold should fail");
    assert!(threshold_err.contains("threshold"));

    let window_err = validate_implicit_review_params(5, 51, 10)
        .expect_err("oversized chapter window should fail");
    assert!(window_err.contains("window_chapters"));

    let evidence_err =
        validate_implicit_review_params(5, 10, 0).expect_err("zero evidence limit should fail");
    assert!(evidence_err.contains("evidence_limit"));
}

#[test]
fn review_rejects_invalid_classification_values() {
    let records = vec![
        behavior(1, 4, json!({"谨慎": "non-conform"})),
        behavior(2, 3, json!({"谨慎": "conform"})),
        behavior(3, 2, json!({"谨慎": true})),
    ];

    let err = build_implicit_attribute_review("谨慎", json!("遇事先观察"), &records, 1, 10, 10)
        .expect_err("invalid classification should fail");

    assert!(err.contains("classification_results.谨慎"));
}

#[test]
fn review_respects_chapter_window() {
    let records = vec![
        behavior(1, 4, json!({"谨慎": "non-conform"})),
        behavior(2, 3, json!({"谨慎": "non-conform"})),
        behavior(3, 2, json!({"谨慎": "non-conform"})),
        behavior(4, 1, json!({"谨慎": "non-conform"})),
    ];

    let stats = build_implicit_attribute_review("谨慎", json!("遇事先观察"), &records, 3, 2, 10)
        .expect("valid records should build review");

    assert_eq!(stats.total, 2);
    assert_eq!(stats.non_conform_count, 2);
    assert!(!stats.threshold_reached);
}

#[test]
fn review_summary_lists_threshold_reached_attributes() {
    let state = json!({
        "implicit_attributes": {
            "谨慎": "遇事先观察",
            "乐观": "相信事情会变好"
        }
    });
    let records = vec![
        behavior(1, 3, json!({"谨慎": "non-conform", "乐观": "conform"})),
        behavior(2, 2, json!({"谨慎": "non-conform", "乐观": "conform"})),
    ];

    let review = build_implicit_review_summary(7, &state, &records, 2, 10, 10)
        .expect("canonical state should build review");

    assert_eq!(review.needs_review, vec!["谨慎"]);
    assert_eq!(review.attributes[0].attribute_key, "谨慎");
}

#[test]
fn review_summary_rejects_missing_implicit_state() {
    let err = build_implicit_review_summary(7, &json!({}), &[], 2, 10, 10)
        .expect_err("missing implicit state should fail");

    assert!(err.contains("implicit_attributes"));
}
