//! 反推请求 payload 合同测试。

use serde_json::json;

use super::{build_reverse_engineer_user_message, reverse_engineer_source_artifact};

#[test]
fn user_message_references_source_artifact() {
    let message = build_reverse_engineer_user_message("reverse_outline", "生成大纲")
        .expect("reverse message should serialize");

    assert!(message.contains(r#""prompt_name": "reverse_outline""#));
    assert!(message.contains("source_artifact"));
    assert!(message.contains(r#""required_root_field": "outlines""#));
    assert!(message.contains(r#""optional_item_fields""#));
    assert!(message.contains(r#""word_count_target""#));
    assert!(message.contains(r#""forbidden_root_fields""#));
    assert!(!message.contains(r#""summaries": []"#));
    assert!(!message.contains(r#""source_text""#));
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
    let message = build_reverse_engineer_user_message("reverse_blueprint", "生成蓝图")
        .expect("reverse message should serialize");

    assert!(message.contains(r#""system_derived_or_downstream_fields_do_not_output""#));
    assert!(message.contains(r#""optional_top_level_fields""#));
    assert!(message.contains(r#""needs_part_outlines""#));
    assert!(
        message
            .contains(r#""full_synopsis": "300-500 个中文字符，保留起承转合但不要展开到章节级""#)
    );
}
