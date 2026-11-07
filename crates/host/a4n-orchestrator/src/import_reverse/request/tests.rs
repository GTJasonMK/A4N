//! 反推请求 payload 合同测试。

use serde_json::{Value, json};

use super::{build_reverse_engineer_user_message, reverse_engineer_source_artifact};
use crate::import_reverse::ReverseEngineeringTarget;

fn message_payload(message: &str) -> Value {
    let payload = a4n_prompt::request::json_user_message_payload(message)
        .expect("user message should contain the canonical JSON payload");
    serde_json::from_str(payload).expect("user message payload should be valid JSON")
}

#[test]
fn user_message_references_source_artifact() {
    let message = build_reverse_engineer_user_message(ReverseEngineeringTarget::Outline)
        .expect("reverse message should serialize");
    let payload = message_payload(&message);

    assert_eq!(payload["task"]["prompt_name"], "reverse_outline");
    assert_eq!(
        payload["context_refs"]["source"],
        "稳定上下文包 source_artifact"
    );
    assert_eq!(
        payload["output_contract"]["required_root_field"],
        "outlines"
    );
    assert!(payload["output_contract"]["optional_item_fields"].is_array());
    assert_eq!(
        payload["output_contract"]["numeric_rules"]["word_count_target"],
        "如输出，必须为正整数"
    );
    assert!(payload["output_contract"]["forbidden_root_fields"].is_array());
    assert!(payload.get("source_text").is_none());
}

#[test]
fn source_artifact_accepts_json_context() {
    let source = reverse_engineer_source_artifact(r#"{"summaries":[]}"#)
        .expect("source artifact should parse json sample");

    assert_eq!(source["source_context"]["summaries"], json!([]));
    assert!(source.get("source_text").is_none());
}

#[test]
fn source_artifact_accepts_plain_text_sample() {
    let source = reverse_engineer_source_artifact("第一章 正文")
        .expect("source artifact should accept plain text sample");

    assert_eq!(source["source_text"], json!("第一章 正文"));
    assert!(source.get("source_context").is_none());
}

#[test]
fn blueprint_user_message_keeps_contract() {
    let message = build_reverse_engineer_user_message(ReverseEngineeringTarget::Blueprint)
        .expect("reverse message should serialize");
    let payload = message_payload(&message);

    assert!(
        payload["output_contract"]["system_derived_or_downstream_fields_do_not_output"].is_array()
    );
    assert!(payload["output_contract"]["optional_top_level_fields"].is_array());
    assert!(
        payload["output_contract"]["optional_top_level_fields"]
            .as_array()
            .is_some_and(|fields| fields.contains(&json!("needs_part_outlines")))
    );
    assert_eq!(
        payload["output_contract"]["length_limits"]["full_synopsis"],
        "300-500 个中文字符，保留起承转合但不要展开到章节级"
    );
}
