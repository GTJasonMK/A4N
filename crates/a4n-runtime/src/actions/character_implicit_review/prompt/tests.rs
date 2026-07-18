use serde_json::json;

use super::builders::build_implicit_update_user_prompt;
use crate::actions::character_implicit_review::types::{
    ImplicitAttributeReviewSummary, ImplicitEvidenceSummary,
};

#[test]
fn update_prompt_serializes_json_fields_without_fallback() {
    let stats = ImplicitAttributeReviewSummary {
        attribute_key: "谨慎".to_string(),
        current_value: json!("遇事先观察"),
        total: 1,
        conform_count: 0,
        non_conform_count: 1,
        conform_rate: 0.0,
        threshold_reached: true,
        evidence: vec![ImplicitEvidenceSummary {
            behavior_id: 1,
            chapter_number: 3,
            behavior_type: "decision".to_string(),
            behavior_description: "主动推进调查".to_string(),
            original_text: "她没有继续等待。".to_string(),
            behavior_tags: json!(["主动"]),
            classification: json!("non-conform"),
            created_at: None,
        }],
    };

    let prompt =
        build_implicit_update_user_prompt("林夏", "谨慎", &json!("遇事先观察"), &stats, 5, 10)
            .expect("valid JSON values should build prompt");

    assert!(prompt.contains(r#""current_value": "遇事先观察""#));
    assert!(prompt.contains(r#""classification": "non-conform""#));
    assert!(prompt.contains(r#""behavior_id": 1"#));
    assert!(!prompt.contains("created_at"));
    assert!(!prompt.contains("你是小说角色档案复审员"));
}
