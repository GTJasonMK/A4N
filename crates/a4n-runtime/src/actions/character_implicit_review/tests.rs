use super::*;
use serde_json::json;
use types::{ImplicitAttributeReviewSummary, ImplicitEvidenceSummary};

fn review_stats() -> ImplicitAttributeReviewSummary {
    ImplicitAttributeReviewSummary {
        attribute_key: "谨慎".to_string(),
        current_value: json!("遇事先观察"),
        total: 1,
        conform_count: 0,
        non_conform_count: 1,
        conform_rate: 0.0,
        threshold_reached: true,
        evidence: vec![ImplicitEvidenceSummary {
            behavior_id: 11,
            chapter_number: 3,
            behavior_type: "decision".to_string(),
            behavior_description: "主动推进调查".to_string(),
            original_text: "她没有继续等待。".to_string(),
            behavior_tags: json!(["主动"]),
            classification: json!("non-conform"),
            created_at: None,
        }],
    }
}

#[test]
fn implicit_update_review_checkpoint_revalidates_response_content() {
    let checkpoint = json!({
        "response_content": r#"{"decision":"modify","reasoning":"连续行为显示更主动","new_value":"主动谨慎","supporting_behavior_ids":[11]}"#,
        "parsed_output": {
            "decision": "modify",
            "reasoning": "连续行为显示更主动",
            "suggested_new_value": "主动谨慎",
            "evidence_summary": "行为11 / 第3章：主动推进调查（non-conform）"
        },
        "usage": null,
        "finish_reason": "stop"});

    let (response_text, usage, finish_reason) = implicit_update_review_checkpoint_response(
        &checkpoint,
        "implicit_update_review:1:谨慎",
        &review_stats(),
    )
    .expect("valid checkpoint should restore");

    assert!(response_text.contains("\"decision\":\"modify\""));
    assert_eq!(usage, None);
    assert_eq!(finish_reason.as_deref(), Some("stop"));
}

#[test]
fn implicit_update_review_checkpoint_rejects_mismatched_parsed_output() {
    let checkpoint = json!({
        "response_content": r#"{"decision":"modify","reasoning":"连续行为显示更主动","new_value":"主动谨慎","supporting_behavior_ids":[11]}"#,
        "parsed_output": {
            "decision": "keep",
            "reasoning": "连续行为显示更主动",
            "suggested_new_value": null,
            "evidence_summary": "行为11 / 第3章：主动推进调查（non-conform）"
        },
        "usage": null,
        "finish_reason": "stop"});

    let err = implicit_update_review_checkpoint_response(
        &checkpoint,
        "implicit_update_review:1:谨慎",
        &review_stats(),
    )
    .expect_err("checkpoint parsed output must match reparsed response");

    assert!(err.contains("重新解析结果不一致"));
}
