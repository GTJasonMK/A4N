use super::super::types::{ImplicitAttributeReviewSummary, ImplicitEvidenceSummary};
use super::*;
use serde_json::json;

fn stats() -> ImplicitAttributeReviewSummary {
    ImplicitAttributeReviewSummary {
        attribute_key: "谨慎".to_string(),
        current_value: json!("遇事先观察"),
        total: 2,
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
                created_at: None,
            },
            ImplicitEvidenceSummary {
                behavior_id: 12,
                chapter_number: 4,
                behavior_type: "action".to_string(),
                behavior_description: "直接进入危险区域".to_string(),
                original_text: "她推门而入。".to_string(),
                behavior_tags: json!(["果断"]),
                classification: json!("non-conform"),
                created_at: None,
            },
        ],
    }
}

#[test]
fn parse_implicit_update_decision_accepts_modify() {
    let parsed = parse_implicit_update_decision(
            r#"{"decision":"modify","reasoning":"连续行为显示更主动","new_value":"主动谨慎","supporting_behavior_ids":[11,12]}"#,
            &stats(),
        )
        .expect("modify decision should parse");

    assert_eq!(parsed.decision, "modify");
    assert_eq!(parsed.suggested_new_value, Some(json!("主动谨慎")));
    assert!(parsed.evidence_summary.contains("行为11 / 第3章"));
}

#[test]
fn parse_implicit_update_decision_rejects_modify_without_value() {
    let err = parse_implicit_update_decision(
        r#"{"decision":"modify","reasoning":"x","new_value":null,"supporting_behavior_ids":[11]}"#,
        &stats(),
    )
    .expect_err("modify without new value should fail");

    assert!(err.contains("new_value"));
}

#[test]
fn parse_implicit_update_decision_rejects_legacy_evidence_summary() {
    let err = parse_implicit_update_decision(
            r#"{"decision":"keep","reasoning":"x","new_value":null,"evidence_summary":"模型自由证据摘要"}"#,
            &stats(),
        )
        .expect_err("legacy evidence_summary must fail");

    assert!(err.contains("evidence_summary 已废弃"));
}

#[test]
fn parse_implicit_update_decision_rejects_unknown_field() {
    let err = parse_implicit_update_decision(
            r#"{"decision":"keep","reasoning":"x","new_value":null,"supporting_behavior_ids":[11],"notes":"多余说明"}"#,
            &stats(),
        )
        .expect_err("unknown implicit update field should fail");

    assert!(err.contains("未支持字段 `notes`"));
}

#[test]
fn parse_implicit_update_decision_rejects_unknown_supporting_behavior_id() {
    let err = parse_implicit_update_decision(
        r#"{"decision":"keep","reasoning":"x","new_value":null,"supporting_behavior_ids":[99]}"#,
        &stats(),
    )
    .expect_err("unknown supporting behavior id must fail");

    assert!(err.contains("不在输入 evidence 候选中"));
}
