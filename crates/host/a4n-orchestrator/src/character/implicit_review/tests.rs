use serde_json::json;

use super::{
    contracts::{BehaviorForReview, ImplicitAttributeReviewSummary, ImplicitEvidenceSummary},
    output::{
        implicit_update_decision_output_json, implicit_update_review_checkpoint_response,
        parse_implicit_update_decision,
    },
    stats::{build_implicit_attribute_review, build_implicit_review_summary},
};
fn behavior(
    id: i64,
    chapter_number: i32,
    classification_results: serde_json::Value,
) -> BehaviorForReview {
    BehaviorForReview {
        behavior_id: id,
        chapter_number,
        behavior_type: "decision".to_string(),
        behavior_description: format!("第{chapter_number}章行为"),
        original_text: "原文证据".to_string(),
        behavior_tags: json!(["标签"]),
        classification_results,
    }
}

fn stats() -> ImplicitAttributeReviewSummary {
    ImplicitAttributeReviewSummary {
        attribute_key: "谨慎".to_string(),
        current_value: json!("遇事先观察"),
        conform_count: 0,
        non_conform_count: 2,
        conform_rate: 0.0,
        threshold_reached: true,
        evidence: vec![
            ImplicitEvidenceSummary {
                behavior_id: 11,
                chapter_number: 3,
                behavior_type: "decision".to_string(),
                behavior_description: "主动推进调查".to_string(),
                original_text: "她没有继续等待。".to_string(),
                behavior_tags: json!(["主动"]),
                classification: json!("non-conform"),
            },
            ImplicitEvidenceSummary {
                behavior_id: 12,
                chapter_number: 4,
                behavior_type: "action".to_string(),
                behavior_description: "直接进入危险区域".to_string(),
                original_text: "她推门而入。".to_string(),
                behavior_tags: json!(["果断"]),
                classification: json!("non-conform"),
            },
        ],
    }
}

#[test]
fn review_rejects_invalid_classification_values() {
    let records = vec![
        behavior(1, 4, json!({"谨慎": "non-conform"})),
        behavior(2, 3, json!({"谨慎": "conform"})),
        behavior(3, 2, json!({"谨慎": true})),
    ];
    let error = build_implicit_attribute_review("谨慎", json!("遇事先观察"), &records, 1, 10, 10)
        .expect_err("invalid classification should fail");
    assert!(error.contains("classification_results.谨慎"));
}

#[test]
fn review_respects_chapter_window() {
    let records = vec![
        behavior(1, 4, json!({"谨慎": "non-conform"})),
        behavior(2, 3, json!({"谨慎": "non-conform"})),
        behavior(3, 2, json!({"谨慎": "non-conform"})),
    ];
    let review = build_implicit_attribute_review("谨慎", json!("遇事先观察"), &records, 3, 2, 10)
        .expect("valid records should build review");
    assert_eq!(review.conform_count + review.non_conform_count, 2);
    assert!(!review.threshold_reached);
}

#[test]
fn review_counts_every_behavior_in_the_last_chapter_without_expanding_evidence() {
    let records = vec![
        behavior(1, 5, json!({"乐观": "conform"})),
        behavior(2, 4, json!({"谨慎": "non-conform"})),
        behavior(3, 4, json!({"谨慎": "conform"})),
        behavior(4, 3, json!({"谨慎": "non-conform"})),
        behavior(5, 3, json!({"谨慎": "non-conform"})),
        behavior(6, 2, json!({"谨慎": "non-conform"})),
    ];
    for (window, expected_non_conform) in [(1, 1), (2, 3)] {
        let review =
            build_implicit_attribute_review("谨慎", json!("遇事先观察"), &records, 3, window, 1)
                .expect("all behaviors in the selected chapters must count");
        assert_eq!(review.conform_count, 1);
        assert_eq!(review.non_conform_count, expected_non_conform);
        assert_eq!(review.threshold_reached, window == 2);
        assert_eq!(review.evidence.len(), 1);
        assert_eq!(review.evidence[0].behavior_id, 2);
    }
}

#[test]
fn review_rejects_invalid_classification_in_the_last_window_chapter() {
    let records = vec![
        behavior(1, 4, json!({"谨慎": "non-conform"})),
        behavior(2, 3, json!({"谨慎": "conform"})),
        behavior(3, 3, json!({"谨慎": true})),
    ];
    let error = build_implicit_attribute_review("谨慎", json!("遇事先观察"), &records, 3, 2, 1)
        .expect_err("the last chapter must not hide invalid classifications");
    assert!(error.contains("行为记录 3"));
}

#[test]
fn review_summary_lists_threshold_reached_attributes() {
    let current_state = json!({
        "implicit_attributes": {
            "谨慎": "遇事先观察",
            "乐观": "相信事情会变好",
        }
    });
    let records = vec![
        behavior(1, 3, json!({"谨慎": "non-conform", "乐观": "conform"})),
        behavior(2, 2, json!({"谨慎": "non-conform", "乐观": "conform"})),
    ];
    let review = build_implicit_review_summary(&current_state, &records, 2, 10, 10)
        .expect("canonical state should build review");
    assert_eq!(review.attributes[0].attribute_key, "谨慎");

    let error = build_implicit_review_summary(&json!({}), &[], 2, 10, 10)
        .expect_err("missing implicit state should fail");
    assert!(error.contains("implicit_attributes"));
}

#[test]
fn parse_decision_enforces_schema_and_evidence_ids() {
    let parsed = parse_implicit_update_decision(
        r#"{"decision":"modify","reasoning":"连续行为显示更主动","new_value":"主动谨慎","supporting_behavior_ids":[11,12]}"#,
        &stats(),
    )
    .expect("modify decision should parse");
    assert_eq!(parsed.suggested_new_value, Some(json!("主动谨慎")));
    assert!(parsed.evidence_summary.contains("行为11 / 第3章"));

    let unknown = parse_implicit_update_decision(
        r#"{"decision":"keep","reasoning":"x","new_value":null,"supporting_behavior_ids":[99]}"#,
        &stats(),
    )
    .expect_err("unknown evidence id must fail");
    assert!(unknown.contains("不在输入 evidence 候选中"));

    let extra = parse_implicit_update_decision(
        r#"{"decision":"keep","reasoning":"x","new_value":null,"supporting_behavior_ids":[11],"notes":"extra"}"#,
        &stats(),
    )
    .expect_err("unknown field must fail");
    assert!(extra.contains("未支持字段 `notes`"));

    let legacy = parse_implicit_update_decision(
        r#"{"decision":"keep","reasoning":"x","new_value":null,"evidence_summary":"模型自由摘要"}"#,
        &stats(),
    )
    .expect_err("legacy evidence summary must fail");
    assert!(legacy.contains("未支持字段 `evidence_summary`"));

    let missing_value = parse_implicit_update_decision(
        r#"{"decision":"modify","reasoning":"x","new_value":null,"supporting_behavior_ids":[11]}"#,
        &stats(),
    )
    .expect_err("modify without value must fail");
    assert!(missing_value.contains("new_value"));
}

#[test]
fn checkpoint_revalidates_response_content() {
    let response_content = r#"{"decision":"modify","reasoning":"连续行为显示更主动","new_value":"主动谨慎","supporting_behavior_ids":[11]}"#;
    let parsed = parse_implicit_update_decision(response_content, &stats()).unwrap();
    let checkpoint = json!({
        "response_content": response_content,
        "parsed_output": implicit_update_decision_output_json(&parsed),
        "usage": null,
        "finish_reason": "stop",
    });
    let restored = implicit_update_review_checkpoint_response(
        &checkpoint["parsed_output"],
        checkpoint["response_content"].as_str().unwrap(),
        "implicit_update_review:1:谨慎",
        &stats(),
    )
    .expect("valid checkpoint should restore");
    assert_eq!(restored.decision.as_str(), "modify");

    let mut mismatched = checkpoint;
    mismatched["parsed_output"]["decision"] = json!("keep");
    let error = implicit_update_review_checkpoint_response(
        &mismatched["parsed_output"],
        mismatched["response_content"].as_str().unwrap(),
        "implicit_update_review:1:谨慎",
        &stats(),
    )
    .expect_err("mismatched parsed output must fail");
    assert!(error.contains("checkpoint_parsed_output_mismatch"));
}
